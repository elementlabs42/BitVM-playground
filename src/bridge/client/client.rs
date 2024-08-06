use musig2::SecNonce;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self},
};

use bitcoin::{absolute::Height, Address, Amount, Network, OutPoint, PublicKey, ScriptBuf, Txid};
use esplora_client::{AsyncClient, Builder, Utxo};

use super::{
    super::{
        contexts::{
            depositor::DepositorContext, operator::OperatorContext, verifier::VerifierContext,
            withdrawer::WithdrawerContext,
        },
        graphs::{
            base::BaseGraph,
            peg_in::{generate_id as peg_in_generate_id, PegInGraph},
            peg_out::{generate_id as peg_out_generate_id, PegOutGraph},
        },
        serialization::{serialize, try_deserialize},
        transactions::base::{Input, InputWithScript},
    },
    data_store::data_store::DataStore,
};

const ESPLORA_URL: &str = "https://mutinynet.com/api";
const TEN_MINUTES: u64 = 10 * 60;

pub type UtxoSet = HashMap<OutPoint, Height>;

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct BitVMClientPublicData {
    pub version: u32,
    pub peg_in_graphs: Vec<PegInGraph>,
    pub peg_out_graphs: Vec<PegOutGraph>,
}

pub struct BitVMClientPrivateData {
    // Peg in and peg out nonces all go into the same file for now
    // Graph ID -> Tx ID -> Input index
    pub secret_nonces: HashMap<String, HashMap<Txid, HashMap<usize, SecNonce>>>,
}

pub struct BitVMClient {
    pub esplora: AsyncClient,

    depositor_context: Option<DepositorContext>,
    operator_context: Option<OperatorContext>,
    verifier_context: Option<VerifierContext>,
    withdrawer_context: Option<WithdrawerContext>,

    data_store: DataStore,
    data: BitVMClientPublicData,
    pub fetched_file_name: Option<String>,

    private_data: BitVMClientPrivateData,
}

impl BitVMClient {
    pub async fn new(
        network: Network,
        n_of_n_public_keys: &Vec<PublicKey>,
        depositor_secret: Option<&str>,
        operator_secret: Option<&str>,
        verifier_secret: Option<&str>,
        withdrawer_secret: Option<&str>,
    ) -> Self {
        let mut depositor_context = None;
        if depositor_secret.is_some() {
            depositor_context = Some(DepositorContext::new(
                network,
                depositor_secret.unwrap(),
                n_of_n_public_keys,
            ));
        }

        let mut operator_context = None;
        if operator_secret.is_some() {
            operator_context = Some(OperatorContext::new(
                network,
                operator_secret.unwrap(),
                n_of_n_public_keys,
            ));
        }

        let mut verifier_context = None;
        if verifier_secret.is_some() {
            verifier_context = Some(VerifierContext::new(
                network,
                verifier_secret.unwrap(),
                n_of_n_public_keys,
            ));
        }

        let mut withdrawer_context = None;
        if withdrawer_secret.is_some() {
            withdrawer_context = Some(WithdrawerContext::new(
                network,
                withdrawer_secret.unwrap(),
                n_of_n_public_keys,
            ));
        }

        let mut data = BitVMClientPublicData {
            version: 1,
            peg_in_graphs: vec![],
            peg_out_graphs: vec![],
        };

        let data_store = DataStore::new();

        // get latest data
        let all_file_names_result = data_store.get_file_names().await;
        let mut latest_file_name: Option<String> = None;
        if all_file_names_result.is_ok() {
            let latest_file: Option<BitVMClientPublicData>;
            (latest_file, latest_file_name) =
                Self::fetch_latest_valid_file(&data_store, &mut all_file_names_result.unwrap())
                    .await;
            if latest_file.is_some() && latest_file_name.is_some() {
                Self::save_local_file(
                    latest_file_name.as_ref().unwrap(),
                    &serialize(latest_file.as_ref().unwrap()),
                );
                data = latest_file.unwrap();
            }
        }

        // TODO: load from local machine
        let private_data = BitVMClientPrivateData {
            secret_nonces: HashMap::new(),
        };

        Self {
            esplora: Builder::new(ESPLORA_URL)
                .build_async()
                .expect("Could not build esplora client"),

            depositor_context,
            operator_context,
            verifier_context,
            withdrawer_context,

            data_store,
            data,
            fetched_file_name: latest_file_name,

            private_data,
        }
    }

    pub async fn sync(&mut self) { self.read().await; }

    pub async fn flush(&mut self) { self.save().await; }

    /*
     1. Fetch the lates file
     2. Fetch all files within 10 minutes (use timestamp)
     3. Merge files
     4. Modify file
     5. Fetch files that was created after fetching 1-2.
     6. Merge with your file
     7. Push the file to the server
    */

    async fn read(&mut self) {
        let latest_file_names_result =
            Self::get_latest_file_names(&self.data_store, self.fetched_file_name.clone()).await;

        if latest_file_names_result.is_ok() {
            let mut latest_file_names = latest_file_names_result.unwrap();
            if !latest_file_names.is_empty() {
                println!("Reading..."); // TODO: remove

                // fetch latest valid file
                println!("****** Try to fetch latest valid file ******"); // TODO: remove
                let (latest_file, latest_file_name) =
                    Self::fetch_latest_valid_file(&self.data_store, &mut latest_file_names).await;
                if latest_file.is_some() && latest_file_name.is_some() {
                    Self::save_local_file(
                        latest_file_name.as_ref().unwrap(),
                        &serialize(&latest_file.as_ref().unwrap()),
                    );
                    Self::merge_data(&mut self.data, latest_file.unwrap());
                    self.fetched_file_name = latest_file_name;

                    // fetch and process all the previous files if latest valid file exists
                    println!("****** Try to fetch and process past files ******"); // TODO: remove
                    let result =
                        Self::process_files_by_timestamp(self, latest_file_names, TEN_MINUTES)
                            .await;
                    match result {
                        Ok(_) => println!("Ok"),
                        Err(err) => println!("Error: {}", err),
                    }
                }
            } else {
                println!("Up to date. No need to read data from the server.");
            }
        } else {
            println!("Error: {}", latest_file_names_result.unwrap_err());
        }
    }

    async fn get_latest_file_names(
        data_store: &DataStore,
        fetched_file_name: Option<String>,
    ) -> Result<Vec<String>, String> {
        let all_file_names_result = data_store.get_file_names().await;
        if all_file_names_result.is_ok() {
            let mut all_file_names = all_file_names_result.unwrap();

            if fetched_file_name.is_some() {
                let fetched_file_position = all_file_names
                    .iter()
                    .position(|file_name| file_name.eq(fetched_file_name.as_ref().unwrap()));
                if fetched_file_position.is_some() {
                    let unfetched_file_position = fetched_file_position.unwrap() + 1;
                    if all_file_names.len() > unfetched_file_position {
                        all_file_names = all_file_names.split_off(unfetched_file_position);
                    } else {
                        all_file_names.clear(); // no files to process
                    }
                }
            }

            return Ok(all_file_names);
        } else {
            return Err(all_file_names_result.unwrap_err());
        }
    }

    async fn filter_files_names_by_timestamp(
        latest_file_names: Vec<String>,
        fetched_file_name: &Option<String>,
        period: u64,
    ) -> Result<Vec<String>, String> {
        if fetched_file_name.is_some() {
            let latest_timestamp =
                DataStore::get_file_timestamp(fetched_file_name.as_ref().unwrap());
            if latest_timestamp.is_err() {
                return Err(latest_timestamp.unwrap_err());
            }

            let past_max_file_name =
                DataStore::get_past_max_file_name_by_timestamp(latest_timestamp.unwrap(), period);

            let mut previous_max_position = latest_file_names
                .iter()
                .position(|file_name| file_name >= &past_max_file_name);
            if previous_max_position.is_none() {
                previous_max_position = Some(latest_file_names.len());
            }

            let file_names_to_process = latest_file_names
                .clone()
                .split_off(previous_max_position.unwrap());

            for file in file_names_to_process.iter() {
                println!("File to process: {}", file);
            }

            return Ok(file_names_to_process);
        } else {
            return Err(String::from(
                "No latest file data. Must fetch the latest file first.",
            ));
        }
    }

    async fn process_files_by_timestamp(
        &mut self,
        latest_file_names: Vec<String>,
        period: u64,
    ) -> Result<String, String> {
        let file_names_to_process_result = Self::filter_files_names_by_timestamp(
            latest_file_names,
            &self.fetched_file_name,
            period,
        )
        .await;

        if file_names_to_process_result.is_err() {
            return Err(file_names_to_process_result.unwrap_err());
        }

        let mut file_names_to_process = file_names_to_process_result.unwrap();
        file_names_to_process.reverse();

        Self::process_files(self, file_names_to_process).await;

        return Ok(String::from("OK"));
    }

    async fn process_files(&mut self, file_names: Vec<String>) -> Option<String> {
        let mut latest_valid_file_name: Option<String> = None;
        if file_names.len() == 0 {
            println!("No additional files to process")
        } else {
            // TODO: can be optimized to fetch all data at once?
            for file_name in file_names.iter() {
                let result = self.data_store.fetch_data_by_key(file_name).await;
                if result.is_ok() && result.as_ref().unwrap().is_some() {
                    println!("Fetched file: {}", file_name); // TODO: remove
                    let data =
                        try_deserialize::<BitVMClientPublicData>(&(result.unwrap()).unwrap());
                    if data.is_ok() && Self::validate_data(&data.as_ref().unwrap()) {
                        // merge the file if the data is valid
                        println!("Merging {} data...", { file_name });
                        Self::merge_data(&mut self.data, data.unwrap());
                        if latest_valid_file_name.is_none() {
                            latest_valid_file_name = Some(file_name.clone());
                        }
                    } else {
                        // skip the file if the data is invalid
                        println!("Invalid file {}, Skipping...", file_name);
                    }
                }
            }
        }

        return latest_valid_file_name;
    }

    async fn fetch_latest_valid_file(
        data_store: &DataStore,
        file_names: &mut Vec<String>,
    ) -> (Option<BitVMClientPublicData>, Option<String>) {
        let mut latest_valid_file: Option<BitVMClientPublicData> = None;
        let mut latest_valid_file_name: Option<String> = None;

        while !file_names.is_empty() {
            let file_name_result = file_names.pop();
            if file_name_result.is_some() {
                let file_name = file_name_result.unwrap();
                let latest_data = Self::fetch_by_key(data_store, &file_name).await;
                if latest_data.is_some() && Self::validate_data(&latest_data.as_ref().unwrap()) {
                    // data is valid
                    println!("Fetched valid file: {}", file_name);
                    latest_valid_file = latest_data;
                    latest_valid_file_name = Some(file_name);
                    break;
                } else {
                    println!("Invalid file: {}", file_name); // TODO: can be removed
                }
                // for invalid data try another file
            }
        }

        return (latest_valid_file, latest_valid_file_name);
    }

    async fn fetch(data_store: &DataStore) -> Option<BitVMClientPublicData> {
        let result = data_store.fetch_latest_data().await;
        if result.is_ok() {
            let json = result.unwrap();
            if json.is_some() {
                let data = try_deserialize::<BitVMClientPublicData>(&json.unwrap());
                if data.is_ok() {
                    return Some(data.unwrap());
                }
            }
        }

        None
    }

    async fn fetch_by_key(data_store: &DataStore, key: &String) -> Option<BitVMClientPublicData> {
        let result = data_store.fetch_data_by_key(key).await;
        if result.is_ok() {
            let json = result.unwrap();
            if json.is_some() {
                let data = try_deserialize::<BitVMClientPublicData>(&json.unwrap());
                if data.is_ok() {
                    return Some(data.unwrap());
                }
            }
        }

        None
    }

    async fn save(&mut self) {
        // read newly created data before pushing
        let latest_file_names_result =
            Self::get_latest_file_names(&self.data_store, self.fetched_file_name.clone()).await;

        if latest_file_names_result.is_ok() {
            let mut latest_file_names = latest_file_names_result.unwrap();
            latest_file_names.reverse();
            let latest_valid_file_name = Self::process_files(self, latest_file_names).await;
            self.fetched_file_name = latest_valid_file_name;
        }

        // push data
        self.data.version += 1;

        let json = serialize(&self.data);
        let result = self.data_store.write_data(json.clone()).await;
        match result {
            Ok(key) => {
                println!("Saved successfully to {}", key);
                Self::save_local_file(&key, &json);
            }
            Err(err) => println!("Failed to save: {}", err),
        }
    }

    fn validate_data(data: &BitVMClientPublicData) -> bool {
        for peg_in_graph in data.peg_in_graphs.iter() {
            if !peg_in_graph.validate() {
                println!(
                    "Encountered invalid peg in graph (Graph id: {})",
                    peg_in_graph.id()
                );
                return false;
            }
        }
        for peg_out_graph in data.peg_out_graphs.iter() {
            if !peg_out_graph.validate() {
                println!(
                    "Encountered invalid peg out graph (Graph id: {})",
                    peg_out_graph.id()
                );
                return false;
            }
        }

        println!("All graph data is valid");
        true
    }

    /// Merges `data` into `local_data`.
    ///
    /// # Arguments
    ///
    /// * `local_data` - Local BitVMClient data.
    /// * `data` - Must be valid data verified via `BitVMClient::validate_data()` function
    fn merge_data(local_data: &mut BitVMClientPublicData, data: BitVMClientPublicData) {
        // peg-in graphs
        let mut peg_in_graphs_by_id: HashMap<String, &mut PegInGraph> = HashMap::new();
        for peg_in_graph in local_data.peg_in_graphs.iter_mut() {
            peg_in_graphs_by_id.insert(peg_in_graph.id().clone(), peg_in_graph);
        }

        let mut peg_in_graphs_to_add: Vec<&PegInGraph> = Vec::new();
        for peg_in_graph in data.peg_in_graphs.iter() {
            let graph = peg_in_graphs_by_id.get_mut(peg_in_graph.id());
            if graph.is_some() {
                graph.unwrap().merge(peg_in_graph);
            } else {
                peg_in_graphs_to_add.push(peg_in_graph);
            }
        }

        for graph in peg_in_graphs_to_add.into_iter() {
            local_data.peg_in_graphs.push(graph.clone());
        }

        // peg-out graphs
        let mut peg_out_graphs_by_id: HashMap<String, &mut PegOutGraph> = HashMap::new();
        for peg_out_graph in local_data.peg_out_graphs.iter_mut() {
            let id = peg_out_graph.id().clone();
            peg_out_graphs_by_id.insert(id, peg_out_graph);
        }

        let mut peg_out_graphs_to_add: Vec<&PegOutGraph> = Vec::new();
        for peg_out_graph in data.peg_out_graphs.iter() {
            let graph = peg_out_graphs_by_id.get_mut(peg_out_graph.id());
            if graph.is_some() {
                graph.unwrap().merge(peg_out_graph);
            } else {
                peg_out_graphs_to_add.push(peg_out_graph);
            }
        }

        for graph in peg_out_graphs_to_add.into_iter() {
            local_data.peg_out_graphs.push(graph.clone());
        }
    }

    // fn process(&self) {
    //     for peg_in_graph in self.data.peg_in_graphs.iter() {
    //         // match graph.get(outpoint) {
    //         //     Some(subsequent_txs) => {
    //         //         for bridge_transaction in subsequent_txs {
    //         //             // TODO: Check whether the transaction is executable
    //         //             let tx = bridge_transaction.finalize();
    //         //             match self.esplora.broadcast(&tx).await {
    //         //                 Ok(_) => {
    //         //                     println!(
    //         //                         "Succesfully broadcast next transaction with id: {}",
    //         //                         tx.compute_txid()
    //         //                     );
    //         //                     remove_utxo = Some(*outpoint);
    //         //                     break;
    //         //                 }
    //         //                 Err(err) => panic!("Tx Broadcast Error: {}", err),
    //         //             }
    //         //         }
    //         //     }
    //         //     None => continue,
    //         // }
    //     }
    // }

    pub async fn status(&self) {
        if self.depositor_context.is_some() {
            self.depositor_status().await;
        }
        if self.operator_context.is_some() {
            self.operator_status().await;
        }
        if self.verifier_context.is_some() {
            self.verifier_status().await;
        }
    }

    async fn depositor_status(&self) {
        if self.depositor_context.is_none() {
            panic!("Depositor context must be initialized");
        }

        let depositor_public_key = &self
            .depositor_context
            .as_ref()
            .unwrap()
            .depositor_public_key;
        for peg_in_graph in self.data.peg_in_graphs.iter() {
            if peg_in_graph.depositor_public_key.eq(depositor_public_key) {
                let status = peg_in_graph.depositor_status(&self.esplora).await;
                println!("Graph id: {} status: {}\n", peg_in_graph.id(), status);
            }
        }
    }

    async fn operator_status(&self) {
        if self.operator_context.is_none() {
            panic!("Operator context must be initialized");
        }

        let mut peg_out_graphs_by_id: HashMap<&String, &PegOutGraph> = HashMap::new();
        for peg_out_graph in self.data.peg_out_graphs.iter() {
            peg_out_graphs_by_id.insert(&peg_out_graph.id(), peg_out_graph);
        }

        let operator_public_key = &self.operator_context.as_ref().unwrap().operator_public_key;
        for peg_in_graph in self.data.peg_in_graphs.iter() {
            let peg_out_graph_id = peg_out_generate_id(peg_in_graph, operator_public_key);
            if !peg_out_graphs_by_id.contains_key(&peg_out_graph_id) {
                println!(
                    "Graph id: {} status: {}\n",
                    peg_in_graph.id(),
                    "Missing peg out graph" // TODO update this to ask the operator to create a new peg out graph
                );
            } else {
                let peg_out_graph = peg_out_graphs_by_id.get(&peg_out_graph_id).unwrap();
                let status = peg_out_graph.operator_status(&self.esplora).await;
                println!("Graph id: {} status: {}\n", peg_out_graph.id(), status);
            }
        }
    }

    async fn verifier_status(&self) {
        if self.verifier_context.is_none() {
            panic!("Verifier context must be initialized");
        }

        for peg_out_graph in self.data.peg_out_graphs.iter() {
            let status = peg_out_graph.verifier_status(&self.esplora).await;
            println!("Graph id: {} status: {}\n", peg_out_graph.id(), status);
        }
    }

    pub async fn create_peg_in_graph(&mut self, input: Input, evm_address: &str) -> String {
        if self.depositor_context.is_none() {
            panic!("Depositor context must be initialized");
        }

        let peg_in_graph =
            PegInGraph::new(self.depositor_context.as_ref().unwrap(), input, evm_address);
        let ret_val = peg_in_graph.id().clone();

        let id = peg_in_generate_id(&peg_in_graph.peg_in_deposit_transaction);
        // TODO broadcast peg in txn

        let graph = self
            .data
            .peg_in_graphs
            .iter()
            .find(|&peg_out_graph| peg_out_graph.id().eq(&id));
        if graph.is_some() {
            panic!("Peg in graph already exists");
        }

        self.data.peg_in_graphs.push(peg_in_graph);

        // self.save().await;

        return id;
    }

    pub async fn broadcast_peg_in_refund(&mut self, peg_in_graph_id: &str) {
        let peg_in_graph = self
            .data
            .peg_in_graphs
            .iter()
            .find(|&peg_in_graph| peg_in_graph.id().eq(peg_in_graph_id));
        if peg_in_graph.is_none() {
            panic!("Invalid graph id");
        }

        // Attempt to broadcast refund tx
    }

    pub async fn create_peg_out_graph(&mut self, peg_in_graph_id: &str, kickoff_input: Input) {
        if self.operator_context.is_none() {
            panic!("Operator context must be initialized");
        }
        let operator_public_key = &self.operator_context.as_ref().unwrap().operator_public_key;

        let peg_in_graph = self
            .data
            .peg_in_graphs
            .iter()
            .find(|&peg_in_graph| peg_in_graph.id().eq(peg_in_graph_id));
        if peg_in_graph.is_none() {
            panic!("Invalid graph id");
        }

        let peg_out_graph_id = peg_out_generate_id(peg_in_graph.unwrap(), operator_public_key);
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter()
            .find(|&peg_out_graph| peg_out_graph.id().eq(&peg_out_graph_id));
        if peg_out_graph.is_some() {
            panic!("Peg out graph already exists");
        }

        let peg_out_graph = PegOutGraph::new(
            self.operator_context.as_ref().unwrap(),
            peg_in_graph.unwrap(),
            kickoff_input,
        );

        // peg_out_graph.kick_off(&self.esplora).await;

        self.data.peg_out_graphs.push(peg_out_graph);

        // self.save().await;
    }

    pub async fn broadcast_kick_off(&mut self, peg_out_graph_id: &str) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph.unwrap().kick_off(&self.esplora).await;
    }

    pub async fn broadcast_challenge(
        &mut self,
        peg_out_graph_id: &str,
        crowdfundng_inputs: &Vec<InputWithScript<'_>>,
        output_script_pubkey: ScriptBuf,
    ) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        if self.depositor_context.is_some() {
            peg_out_graph
                .unwrap()
                .challenge(
                    &self.esplora,
                    self.depositor_context.as_ref().unwrap(),
                    crowdfundng_inputs,
                    &self.depositor_context.as_ref().unwrap().depositor_keypair,
                    output_script_pubkey,
                )
                .await;
        } else if self.operator_context.is_some() {
            peg_out_graph
                .unwrap()
                .challenge(
                    &self.esplora,
                    self.operator_context.as_ref().unwrap(),
                    crowdfundng_inputs,
                    &self.operator_context.as_ref().unwrap().operator_keypair,
                    output_script_pubkey,
                )
                .await;
        } else if self.verifier_context.is_some() {
            peg_out_graph
                .unwrap()
                .challenge(
                    &self.esplora,
                    self.verifier_context.as_ref().unwrap(),
                    crowdfundng_inputs,
                    &self.verifier_context.as_ref().unwrap().verifier_keypair,
                    output_script_pubkey,
                )
                .await;
        } else if self.withdrawer_context.is_some() {
            peg_out_graph
                .unwrap()
                .challenge(
                    &self.esplora,
                    self.withdrawer_context.as_ref().unwrap(),
                    crowdfundng_inputs,
                    &self.withdrawer_context.as_ref().unwrap().withdrawer_keypair,
                    output_script_pubkey,
                )
                .await;
        }
    }

    pub async fn broadcast_assert(&mut self, peg_out_graph_id: &str) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph.unwrap().assert(&self.esplora).await;
    }

    pub async fn broadcast_disprove(
        &mut self,
        peg_out_graph_id: &str,
        input_script_index: u32,
        output_script_pubkey: ScriptBuf,
    ) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph
            .unwrap()
            .disprove(&self.esplora, input_script_index, output_script_pubkey)
            .await;
    }

    pub async fn broadcast_burn(
        &mut self,
        peg_out_graph_id: &str,
        output_script_pubkey: ScriptBuf,
    ) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph
            .unwrap()
            .burn(&self.esplora, output_script_pubkey)
            .await;
    }

    pub async fn broadcast_take1(&mut self, peg_out_graph_id: &str) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph.unwrap().take1(&self.esplora).await;
    }

    pub async fn broadcast_take2(&mut self, peg_out_graph_id: &str) {
        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph.unwrap().take2(&self.esplora).await;
    }

    pub async fn get_initial_utxo(&self, address: Address, amount: Amount) -> Option<Utxo> {
        let utxos = self.esplora.get_address_utxo(address).await.unwrap();
        let possible_utxos = utxos
            .into_iter()
            .filter(|utxo| utxo.value == amount)
            .collect::<Vec<_>>();
        if !possible_utxos.is_empty() {
            Some(possible_utxos[0].clone())
        } else {
            None
        }
    }

    pub async fn get_initial_utxos(&self, address: Address, amount: Amount) -> Option<Vec<Utxo>> {
        let utxos = self.esplora.get_address_utxo(address).await.unwrap();
        let possible_utxos = utxos
            .into_iter()
            .filter(|utxo| utxo.value == amount)
            .collect::<Vec<_>>();
        if !possible_utxos.is_empty() {
            Some(possible_utxos)
        } else {
            None
        }
    }

    pub fn push_peg_in_nonces(&mut self, peg_in_graph_id: &str) {
        if self.verifier_context.is_none() {
            panic!("Can only be called by a verifier!");
        }

        let peg_in_graph = self
            .data
            .peg_in_graphs
            .iter_mut()
            .find(|peg_in_graph| peg_in_graph.id().eq(peg_in_graph_id));
        if peg_in_graph.is_none() {
            panic!("Invalid graph id");
        }

        let secret_nonces = peg_in_graph
            .unwrap()
            .push_nonces(&self.verifier_context.as_ref().unwrap());

        if self
            .private_data
            .secret_nonces
            .get(peg_in_graph_id)
            .is_none()
        {
            self.private_data
                .secret_nonces
                .insert(peg_in_graph_id.to_string(), HashMap::new());
        }
        self.private_data
            .secret_nonces
            .get_mut(peg_in_graph_id)
            .unwrap()
            .extend(secret_nonces);

        // TODO: Save secret nonces for all txs in the graph to the local file system. Later, when pre-signing the tx,
        // we'll need to retrieve these nonces for this graph ID.
    }

    pub fn push_peg_out_nonces(&mut self, peg_out_graph_id: &str) {
        if self.verifier_context.is_none() {
            panic!("Can only be called by a verifier!");
        }

        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        let secret_nonces = peg_out_graph
            .unwrap()
            .push_nonces(&self.verifier_context.as_ref().unwrap());

        if self
            .private_data
            .secret_nonces
            .get(peg_out_graph_id)
            .is_none()
        {
            self.private_data
                .secret_nonces
                .insert(peg_out_graph_id.to_string(), HashMap::new());
        }
        self.private_data
            .secret_nonces
            .get_mut(peg_out_graph_id)
            .unwrap()
            .extend(secret_nonces);

        // TODO: Save secret nonces for all txs in the graph to the local file system. Later, when pre-signing the tx,
        // we'll need to retrieve these nonces for this graph ID.

        // TODO: Add public nonces in the remaining txs in this graph.
    }

    pub fn pre_sign_peg_in(&mut self, peg_in_graph_id: &str) {
        if self.operator_context.is_none() && self.verifier_context.is_none() {
            panic!("Can only be called by an operator or a verifier!");
        }

        let peg_in_graph = self
            .data
            .peg_in_graphs
            .iter_mut()
            .find(|peg_in_graph| peg_in_graph.id().eq(peg_in_graph_id));
        if peg_in_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_in_graph.unwrap().pre_sign(
            &self.verifier_context.as_ref().unwrap(),
            &self.private_data.secret_nonces[peg_in_graph_id],
        );
    }

    pub fn pre_sign_peg_out(&mut self, peg_out_graph_id: &str) {
        if self.operator_context.is_none() && self.verifier_context.is_none() {
            panic!("Can only be called by an operator or a verifier!");
        }

        let peg_out_graph = self
            .data
            .peg_out_graphs
            .iter_mut()
            .find(|peg_out_graph| peg_out_graph.id().eq(peg_out_graph_id));
        if peg_out_graph.is_none() {
            panic!("Invalid graph id");
        }

        peg_out_graph.unwrap().pre_sign(
            &self.verifier_context.as_ref().unwrap(),
            &self.private_data.secret_nonces[peg_out_graph_id],
        );
    }

    fn save_local_file(key: &String, json: &String) {
        println!("Saving local file {}", key);
        fs::write(format!("results/{}", key), json).expect("Unable to write a file");
    }

    // pub async fn execute_possible_txs(
    //     &mut self,
    //     context: &dyn BaseContext,
    //     graph: &mut CompiledBitVMGraph,
    // ) {
    //     // Iterate through our UTXO set and execute an executable TX
    //     // TODO: May have to respect an order here.
    //     let mut remove_utxo = None;
    //     for (outpoint, _) in self.utxo_set.iter() {
    //         match graph.get(outpoint) {
    //             Some(subsequent_txs) => {
    //                 for bridge_transaction in subsequent_txs {
    //                     // TODO: Check whether the transaction is executable
    //                     let tx = bridge_transaction.finalize();
    //                     match self.esplora.broadcast(&tx).await {
    //                         Ok(_) => {
    //                             println!(
    //                                 "Succesfully broadcast next transaction with id: {}",
    //                                 tx.compute_txid()
    //                             );
    //                             remove_utxo = Some(*outpoint);
    //                             break;
    //                         }
    //                         Err(err) => panic!("Tx Broadcast Error: {}", err),
    //                     }
    //                 }
    //             }
    //             None => continue,
    //         }
    //     }

    //     if let Some(remove_utxo) = remove_utxo {
    //         self.utxo_set.remove(&remove_utxo);
    //         graph.remove(&remove_utxo);
    //     }
    // }

    // pub async fn listen(
    //     &mut self,
    //     context: &dyn BaseContext,
    //     initial_outpoint: OutPoint,
    //     graph: &mut CompiledBitVMGraph,
    // ) {
    //     let builder = Builder::new(ESPLORA_URL);
    //     let esplora = builder.build_async().unwrap();
    //     let mut latest_hash =
    //         BlockHash::from_str("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
    //             .unwrap();
    //     self.utxo_set.insert(initial_outpoint, Height::ZERO);

    //     while !graph.is_empty() {
    //         if let Ok(block_hash) = esplora.get_tip_hash().await {
    //             if block_hash == latest_hash {
    //                 sleep(Duration::from_secs(10));
    //                 continue;
    //             }
    //             latest_hash = block_hash;
    //             // TODO: This assumes that the tip did not increase. There should be a
    //             // better API endpoint like /block-height/{block_hash}
    //             let block_height = esplora.get_height().await.unwrap();
    //             let block = esplora
    //                 .get_block_by_hash(&block_hash)
    //                 .await
    //                 .unwrap()
    //                 .unwrap();

    //             // Handle new block received logic
    //             println!("Received block {}", block_hash);

    //             for tx in block.txdata {
    //                 for (vout, _) in tx.output.iter().enumerate() {
    //                     let outpoint = OutPoint {
    //                         txid: tx.compute_txid(),
    //                         vout: vout as u32,
    //                     };
    //                     if graph.contains_key(&outpoint) {
    //                         // Update our UTXO set
    //                         self.utxo_set
    //                             .insert(outpoint, Height::from_consensus(block_height).unwrap());
    //                     }
    //                 }
    //             }
    //             self.execute_possible_txs(context, graph).await;
    //         }
    //     }
    // }
}
