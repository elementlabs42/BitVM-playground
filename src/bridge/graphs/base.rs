use std::str::FromStr;

use bitcoin::{Network, PublicKey, Txid};
use esplora_client::{AsyncClient, Error};

pub const GRAPH_VERSION: &str = "0.1";

pub const INITIAL_AMOUNT: u64 = 100_000;
pub const FEE_AMOUNT: u64 = 1_000;
pub const DUST_AMOUNT: u64 = 10_000;
pub const ONE_HUNDRED: u64 = 100_000_000;

// TODO delete
// DEMO SECRETS
pub const OPERATOR_SECRET: &str =
    "3076ca1dfc1e383be26d5dd3c0c427340f96139fa8c2520862cf551ec2d670ac";
pub const OPERATOR_PUBKEY: &str =
    "03484db4a2950d63da8455a1b705b39715e4075dd33511d0c7e3ce308c93449deb";
pub const VERIFIER0_SECRET: &str =
    "ee0817eac0c13aa8ee2dd3256304041f09f0499d1089b56495310ae8093583e2";
pub const VERIFIER0_PUBKEY: &str =
    "026cc14f56ad7e8fdb323378287895c6c0bcdbb37714c74fba175a0c5f0cd0d56f";
pub const VERIFIER1_SECRET: &str =
    "fc294c70faf210d4d0807ea7a3dba8f7e41700d90c119e1ae82a0687d89d297f";
pub const VERIFIER1_PUBKEY: &str =
    "02452556ed6dbac394cbb7441fbaf06c446d1321467fa5a138895c6c9e246793dd";
pub const N_OF_N_SECRET: &str = "a9bd8b8ade888ed12301b21318a3a73429232343587049870132987481723497";
pub const N_OF_N_PUBKEYS: [&str; 2] = [VERIFIER0_PUBKEY, VERIFIER1_PUBKEY];
pub const N_OF_N_PUBKEY: &str =
    "022976898dbc2f357d50e113014b0ecc88d488a5aaf67aa1ec95fb60deba3bdfd4";
pub const DEPOSITOR_SECRET: &str =
    "b8f17ea979be24199e7c3fec71ee88914d92fd4ca508443f765d56ce024ef1d7";
pub const WITHDRAWER_SECRET: &str =
    "fffd54f6d8f8ad470cb507fd4b6e9b3ea26b4221a4900cc5ad5916ce67c02f1e";

pub const EVM_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

pub trait BaseGraph {
    fn network(&self) -> Network;
    fn id(&self) -> &String;
}

pub async fn get_block_height(client: &AsyncClient) -> u32 {
    let blockchain_height_result = client.get_height().await;
    if blockchain_height_result.is_err() {
        panic!(
            "Failed to fetch blockchain height! Error occurred {:?}",
            blockchain_height_result
        );
    }

    blockchain_height_result.unwrap()
}

pub async fn verify_if_not_mined(client: &AsyncClient, txid: Txid) {
    let tx_status = client.get_tx_status(&txid).await;
    if tx_status.as_ref().is_ok_and(|status| status.confirmed) {
        panic!("Transaction already mined!");
    } else if tx_status.is_err() {
        panic!(
            "Failed to get transaction status, error occurred {:?}",
            tx_status
        );
    }
}

pub fn verify_tx_result(tx_result: &Result<(), Error>) {
    if tx_result.is_ok() {
        println!("Tx mined successfully.");
    } else {
        panic!("Error occurred {:?}", tx_result);
    }
}
