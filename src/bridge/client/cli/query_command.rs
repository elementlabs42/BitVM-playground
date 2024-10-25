use alloy::primitives::Address;
use bitcoin::{Amount, Denomination, Network, OutPoint, PublicKey, XOnlyPublicKey};
use clap::{arg, ArgMatches, Command};
use core::str::FromStr;

use super::query_response::{Response, ResponseStatus};
use crate::bridge::{
    client::{client::BitVMClient, sdk::query::GraphQuery},
    constants::DestinationNetwork,
    contexts::base::generate_keys_from_secret,
    graphs::base::{VERIFIER_0_SECRET, VERIFIER_1_SECRET},
    transactions::base::Input,
};

pub struct QueryCommand {
    client: BitVMClient,
}

pub const FAKE_SECRET: &str = "1000000000000000000000000000000000000000000000000000000000000000";

impl QueryCommand {
    pub async fn new(
        source_network: Network,
        destination_network: DestinationNetwork,
        path_prefix: Option<&str>,
    ) -> Self {
        let (_, _, verifier_0_public_key) =
            generate_keys_from_secret(Network::Bitcoin, VERIFIER_0_SECRET);
        let (_, _, verifier_1_public_key) =
            generate_keys_from_secret(Network::Bitcoin, VERIFIER_1_SECRET);

        let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
        n_of_n_public_keys.push(verifier_0_public_key);
        n_of_n_public_keys.push(verifier_1_public_key);

        let bitvm_client = BitVMClient::new(
            source_network,
            destination_network,
            &n_of_n_public_keys,
            Some(FAKE_SECRET),
            Some(FAKE_SECRET),
            Some(FAKE_SECRET),
            Some(FAKE_SECRET),
            path_prefix,
        )
        .await;

        Self {
            client: bitvm_client,
        }
    }

    async fn sync(&mut self) {
        self.client.sync().await;
        self.client.sync_l2().await;
    }

    pub fn depositor_command() -> Command {
        Command::new("depositor")
            .about("fetch peg-in graphs related to the specified depositor")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
    }

    pub async fn handle_depositor_command(&mut self, sub_matches: &ArgMatches) -> Response {
        let pubkey = PublicKey::from_str(
            sub_matches
                .get_one::<String>("DEPOSITOR_PUBLIC_KEY")
                .unwrap(),
        );
        if pubkey.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid public key. Use bitcoin public key format."
                )),
                None,
            );
        }

        self.sync().await;
        let result = self
            .client
            .get_depositor_status(&pubkey.clone().unwrap())
            .await;
        if result.len() > 0 {
            let data = Some(serde_json::to_value(result).expect("Failed to merge value vector"));
            return Response::new(ResponseStatus::OK, data);
        } else {
            return Response::new(ResponseStatus::NOK(format!("Depositor not found.")), None);
        }
    }

    pub fn withdrawer_command() -> Command {
        Command::new("withdrawer")
            .about("fetch peg-out graphs related to the specified withdrawer")
            .arg(arg!(<WITHDRAWER_CHAIN_ADDRESS> "WITHDRAWER L2 Chain address").required(true))
    }

    pub async fn handle_withdrawer_command(
        &mut self,
        sub_matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let chain_address = Address::from_str(
            sub_matches
                .get_one::<String>("WITHDRAWER_CHAIN_ADDRESS")
                .unwrap(),
        );
        if chain_address.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid chain address. Use {} address format.",
                    destination_network
                )),
                None,
            );
        }

        self.sync().await;
        let result = self
            .client
            .get_withdrawer_status(&chain_address.unwrap().to_string().as_str())
            .await;
        if result.len() > 0 {
            let data = Some(serde_json::to_value(result).expect("Failed to merge value vector"));
            return Response::new(ResponseStatus::OK, data);
        } else {
            return Response::new(ResponseStatus::NOK(format!("Withdrawer not found.")), None);
        }
    }

    pub fn history_command() -> Command {
        Command::new("history")
            .about("fetch peg-in / peg-out graphs with bitcoin public key and ethereum address at the same time")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .arg(arg!(<WITHDRAWER_CHAIN_ADDRESS> "WITHDRAWER L2 Chain address").required(true))
    }

    pub async fn handle_history_command(
        &mut self,
        sub_matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let pubkey = PublicKey::from_str(
            sub_matches
                .get_one::<String>("DEPOSITOR_PUBLIC_KEY")
                .unwrap(),
        );
        if pubkey.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid public key. Use bitcoin public key format."
                )),
                None,
            );
        }
        let chain_address = Address::from_str(
            sub_matches
                .get_one::<String>("WITHDRAWER_CHAIN_ADDRESS")
                .unwrap(),
        );
        if chain_address.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid chain address. Use {} address format.",
                    destination_network
                )),
                None,
            );
        }

        self.sync().await;
        let mut result_depositor = self
            .client
            .get_depositor_status(&pubkey.clone().unwrap())
            .await;
        let mut result_withdrawer = self
            .client
            .get_withdrawer_status(&chain_address.unwrap().to_string().as_str())
            .await;

        let result = match (result_depositor.len(), result_withdrawer.len()) {
            (0, 0) => vec![],
            (0, _) => result_withdrawer,
            (_, 0) => result_depositor,
            _ => {
                result_depositor.append(&mut result_withdrawer);
                result_depositor
            }
        };

        if result.len() > 0 {
            let data = Some(serde_json::to_value(result).expect("Failed to merge value vector"));
            return Response::new(ResponseStatus::OK, data);
        } else {
            return Response::new(ResponseStatus::NOK(format!("Withdrawer not found.")), None);
        }
    }

    pub fn transactions_command() -> Command {
        Command::new("transactions")
            .about("create transactions of peg-in graph for depositor to sign")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .arg(arg!(<WITHDRAWER_CHAIN_ADDRESS> "WITHDRAWER L2 Chain address").required(true))
            .arg(arg!(<OUTPOINT> "Previous output for peg-in deposit transaction input, format: <txid>:<vout>").required(true))
            .arg(arg!(<SATS> "Amount of satoshis to deposit, should be also the value of previous output").required(true))
    }

    pub async fn handle_transactions_command(
        &self,
        sub_matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let pubkey = PublicKey::from_str(
            sub_matches
                .get_one::<String>("DEPOSITOR_PUBLIC_KEY")
                .unwrap(),
        );
        if pubkey.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid public key. Use bitcoin public key format."
                )),
                None,
            );
        }
        let x_only_pubkey = XOnlyPublicKey::from(pubkey.clone().unwrap());
        let chain_address = Address::from_str(
            sub_matches
                .get_one::<String>("WITHDRAWER_CHAIN_ADDRESS")
                .unwrap(),
        );
        if chain_address.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid chain address. Use {} address format.",
                    destination_network
                )),
                None,
            );
        }
        let outpoint = OutPoint::from_str(sub_matches.get_one::<String>("OUTPOINT").unwrap());
        if outpoint.is_err() {
            return Response::new(
                ResponseStatus::NOK("Invalid OutPoint. Use <txid>:<vout> format.".to_string()),
                None,
            );
        }
        let satoshis = Amount::from_str_in(
            sub_matches.get_one::<String>("SATS").unwrap(),
            Denomination::Satoshi,
        );
        if satoshis.is_err() {
            return Response::new(
                ResponseStatus::NOK("Invalid amount of satoshis. Use u64.".to_string()),
                None,
            );
        }

        // do not need to sync
        let result = self
            .client
            .get_depositor_transactions(
                &pubkey.clone().unwrap(),
                &x_only_pubkey,
                Input {
                    outpoint: outpoint.unwrap(),
                    amount: satoshis.unwrap(),
                },
                &chain_address.unwrap().to_string().as_str(),
            )
            .await;
        Response::new(ResponseStatus::OK, Some(result))
    }
}
