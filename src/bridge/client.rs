use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, thread::sleep, time::Duration};

use bitcoin::{absolute::Height, Address, Amount, Network, OutPoint};
use esplora_client::{AsyncClient, BlockHash, Builder, Utxo};

use super::{
    aws_s3::{self, AwsS3},
    contexts::{
        base::{generate_keys_from_secret, BaseContext},
        depositor::DepositorContext,
        operator::OperatorContext,
        verifier::VerifierContext,
        withdrawer::WithdrawerContext,
    },
    graphs::{
        base::{
            BaseGraph, DEPOSITOR_SECRET, EVM_ADDRESS, N_OF_N_SECRET, OPERATOR_SECRET,
            WITHDRAWER_SECRET,
        },
        peg_in::PegInGraph,
        peg_out::{generate_id, PegOutGraph},
    },
    serialization::{deserialize, serialize},
    transactions::base::Input,
};

const ESPLORA_URL: &str = "https://mutinynet.com/api";

pub type UtxoSet = HashMap<OutPoint, Height>;

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct BitVMClientData {
    pub version: u32,
    pub peg_in_graphs: Vec<PegInGraph>,
    pub peg_out_graphs: Vec<PegOutGraph>,
}

pub struct BitVMClient {
    pub esplora: AsyncClient,

    // Maps OutPoints to their (potentially unconfirmed) UTXOs.
    utxo_set: UtxoSet,

    depositor_context: Option<DepositorContext>,
    operator_context: Option<OperatorContext>,
    verifier_context: Option<VerifierContext>,
    withdrawer_context: Option<WithdrawerContext>,

    data: BitVMClientData,

    aws_s3: AwsS3,
}

impl BitVMClient {
    pub async fn new(
        network: Network,
        depositor_secret: Option<&str>,
        operator_secret: Option<&str>,
        n_of_n_secret: Option<&str>,
        withdrawer_secret: Option<&str>,
    ) -> Self {
        // TODO these publc key values should be hardcoded
        let depositor_keys = generate_keys_from_secret(network, DEPOSITOR_SECRET);
        let operator_keys = generate_keys_from_secret(network, OPERATOR_SECRET);
        let verifier_keys = generate_keys_from_secret(network, N_OF_N_SECRET);
        let withdrawer_keys = generate_keys_from_secret(network, WITHDRAWER_SECRET);

        let mut depositor_context = None;
        if depositor_secret.is_some() {
            depositor_context = Some(DepositorContext::new(
                network,
                depositor_secret.unwrap(),
                &verifier_keys.2,
                &verifier_keys.3,
            ));
        }

        let mut operator_context = None;
        if operator_secret.is_some() {
            operator_context = Some(OperatorContext::new(
                network,
                operator_secret.unwrap(),
                &verifier_keys.2,
                &verifier_keys.3,
            ));
        }

        let mut verifier_context = None;
        if n_of_n_secret.is_some() {
            verifier_context = Some(VerifierContext::new(
                network,
                n_of_n_secret.unwrap(),
                &operator_keys.2,
                &operator_keys.3,
            ));
        }

        let mut withdrawer_context = None;
        if withdrawer_secret.is_some() {
            withdrawer_context = Some(WithdrawerContext::new(
                network,
                withdrawer_secret.unwrap(),
                &verifier_keys.2,
                &verifier_keys.3,
            ));
        }

        let mut data = BitVMClientData {
            version: 1,
            peg_in_graphs: vec![],
            peg_out_graphs: vec![],
        };

        let aws_s3 = AwsS3::new();
        let fetched_data = Self::fetch(&aws_s3).await;
        if fetched_data.is_some() {
            data = fetched_data.unwrap();
        }

        Self {
            esplora: Builder::new(ESPLORA_URL)
                .build_async()
                .expect("Could not build esplora client"),

            utxo_set: UtxoSet::new(),

            depositor_context,
            operator_context,
            verifier_context,
            withdrawer_context,

            data,

            aws_s3,
        }
    }

    pub async fn sync(&mut self) {
        self.read().await;
        self.save().await;
    }

    async fn read(&mut self) {
        let data = Self::fetch(&self.aws_s3).await;
        if data.is_some() {
            self.data = data.unwrap();
        }
    }

    async fn fetch(aws_s3: &AwsS3) -> Option<BitVMClientData> {
        aws_s3.list_objects().await;

        let json = "";
        Some(deserialize::<BitVMClientData>(&json))
    }

    async fn save(&self) { let json = serialize(&self.data); }

    fn verify_data(&self, data: &BitVMClientData) {
        for peg_in_graph in data.peg_in_graphs.iter() {
            self.verify_peg_in_graph(peg_in_graph);
        }
        for peg_out_graph in data.peg_out_graphs.iter() {
            self.verify_peg_out_graph(peg_out_graph);
        }
    }

    fn verify_peg_in_graph(&self, peg_in_graph: &PegInGraph) {}

    fn verify_peg_out_graph(&self, peg_out_graph: &PegOutGraph) {}

    fn process(&self) {
        for peg_in_graph in self.data.peg_in_graphs.iter() {
            // match graph.get(outpoint) {
            //     Some(subsequent_txs) => {
            //         for bridge_transaction in subsequent_txs {
            //             // TODO: Check whether the transaction is executable
            //             let tx = bridge_transaction.finalize();
            //             match self.esplora.broadcast(&tx).await {
            //                 Ok(_) => {
            //                     println!(
            //                         "Succesfully broadcast next transaction with id: {}",
            //                         tx.compute_txid()
            //                     );
            //                     remove_utxo = Some(*outpoint);
            //                     break;
            //                 }
            //                 Err(err) => panic!("Tx Broadcast Error: {}", err),
            //             }
            //         }
            //     }
            //     None => continue,
            // }
        }
    }

    pub fn status(&self) {
        if self.depositor_context.is_some() {
            self.depositor_status();
        }
        if self.operator_context.is_some() {
            self.operator_status();
        }
        if self.verifier_context.is_some() {
            self.verifier_status();
        }
    }

    fn depositor_status(&self) {
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
                println!("Graph id: {:?} status: {:?}\n", peg_in_graph.id(), "todo");
                // If peg in is complete, let depositor know
                // If peg in refund has elapsed, let depositor know
            }
        }
    }

    fn operator_status(&self) {
        if self.operator_context.is_none() {
            panic!("Operator context must be initialized");
        }

        let mut peg_out_graphs_by_id: HashMap<&String, &PegOutGraph> = HashMap::new();
        for peg_out_graph in self.data.peg_out_graphs.iter() {
            peg_out_graphs_by_id.insert(peg_out_graph.id(), peg_out_graph);
        }

        let operator_public_key = &self.operator_context.as_ref().unwrap().operator_public_key;
        for peg_in_graph in self.data.peg_in_graphs.iter() {
            let mut status = "todo";
            let peg_out_graph_id = generate_id(peg_in_graph, operator_public_key);
            if !peg_out_graphs_by_id.contains_key(&peg_out_graph_id) {
                status = "Missing peg out graph";
            }
            // Send kick off txn if needed
            // Assert
            // Take 1
            // Take 2

            println!("Graph id: {:?} status: {:?}\n", peg_in_graph.id(), status);
        }
    }

    fn verifier_status(&self) {
        if self.verifier_context.is_none() {
            panic!("Verifier context must be initialized");
        }

        let n_of_n_public_key = &self.verifier_context.as_ref().unwrap().n_of_n_public_key;
        for peg_out_graph in self.data.peg_out_graphs.iter() {
            // verify graph
            // check if pre-signed
            // check if dispute
            // check if burn
        }
    }

    pub async fn create_peg_in_graph(&mut self, input: Input, evm_address: &str) {
        if self.depositor_context.is_none() {
            panic!("Depositor context must be initialized");
        }

        let peg_in_graph =
            PegInGraph::new(self.depositor_context.as_ref().unwrap(), input, evm_address);

        // TODO broadcast peg in txn

        self.data.peg_in_graphs.push(peg_in_graph);

        self.save();
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

        let peg_out_graph_id = generate_id(peg_in_graph.unwrap(), operator_public_key);
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

        // TODO broadcast kick off txn

        self.data.peg_out_graphs.push(peg_out_graph);

        self.save();
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
