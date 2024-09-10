use std::time::Duration;

use alloy::transports::http::{
    reqwest::{Error, Response, StatusCode},
    Client,
};
use bitcoin::{Address, Amount, OutPoint, Txid};

use bitvm::bridge::client::client::BitVMClient;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

pub const TX_WAIT_TIME: u64 = 45; // in seconds
pub const ESPLORA_FUNDING_URL: &str = "https://faucet.mutinynet.com/";
pub const ESPLORA_RETRIES: usize = 3;
pub const ESPLORA_RETRY_WAIT_TIME: u64 = 5;

pub async fn generate_stub_outpoint(
    client: &BitVMClient,
    funding_utxo_address: &Address,
    input_value: Amount,
) -> OutPoint {
    let funding_utxo = client
        .get_initial_utxo(funding_utxo_address.clone(), input_value)
        .await
        .unwrap_or_else(|| {
            panic!(
                "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                funding_utxo_address,
                input_value.to_sat()
            );
        });
    OutPoint {
        txid: funding_utxo.txid,
        vout: funding_utxo.vout,
    }
}

#[derive(Serialize, Deserialize)]
struct FundResult {
    txid: Txid,
    address: String,
}

async fn fund_input_http(address: &Address, amount: Amount) -> Result<Response, Error> {
    let client = Client::builder()
        .build()
        .expect("Unable to build reqwest client");
    let payload = format!(
        "{{\"sats\":{},\"address\":\"{}\"}}",
        amount.to_sat(),
        address
    );

    println!(
        "Funding {:?} with {} sats at https://faucet.mutinynet.com/",
        address,
        amount.to_sat()
    );
    let resp = client
        .post(format!("{}api/onchain", ESPLORA_FUNDING_URL))
        .body(payload)
        .header("CONTENT-TYPE", "application/json")
        .send()
        .await;

    match resp {
        Ok(resp) => Ok(resp),
        Err(e) => Err(e),
    }
}

pub async fn fund_input(address: &Address, amount: Amount) -> Txid {
    let client_err_handler = |e: Error| {
        panic!("Could not fund {} due to {:?}", address, e);
    };
    let mut resp = fund_input_http(address, amount)
        .await
        .unwrap_or_else(client_err_handler);

    let mut retry = ESPLORA_RETRIES;
    while resp.status().eq(&StatusCode::SERVICE_UNAVAILABLE) && retry > 0 {
        eprintln!("Retrying({}/{}) {:?}...", retry, ESPLORA_RETRIES, address);
        retry -= 1;
        sleep(Duration::from_secs(ESPLORA_RETRY_WAIT_TIME)).await;
        resp = fund_input_http(address, amount)
            .await
            .unwrap_or_else(client_err_handler);
    }

    if resp.status().is_client_error() || resp.status().is_server_error() {
        panic!(
            "Could not fund {} with respond code {:?}",
            address,
            resp.status()
        );
    }

    let result = resp.json::<FundResult>().await.unwrap();
    println!("Funded at: {}", result.txid);

    result.txid
}

pub async fn fund_inputs(inputs_to_fund: &Vec<(&Address, Amount)>) {
    for input in inputs_to_fund {
        fund_input(input.0, input.1).await;
        sleep(Duration::from_micros(TX_WAIT_TIME)).await;
    }
}

fn get_random_seconds(from: u64, to: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(from..to)
}

pub async fn verify_and_fund_inputs(
    client: &BitVMClient,
    funding_inputs: &Vec<(&Address, Amount)>,
) {
    for input in funding_inputs {
        if client
            .get_initial_utxo(input.0.clone(), input.1)
            .await
            .is_none()
        {
            sleep(Duration::from_secs(get_random_seconds(500, 2000))).await;
            fund_input(input.0, input.1).await;
        }
    }
}

pub async fn verify_funding_inputs(client: &BitVMClient, funding_inputs: &Vec<(&Address, Amount)>) {
    let mut inputs_to_fund: Vec<(&Address, Amount)> = vec![];

    for funding_input in funding_inputs {
        if client
            .get_initial_utxo(funding_input.0.clone(), funding_input.1)
            .await
            .is_none()
        {
            inputs_to_fund.push((funding_input.0, funding_input.1));
        }
    }

    for input_to_fund in inputs_to_fund.clone() {
        println!(
            "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
            input_to_fund.0,
            input_to_fund.1.to_sat()
        );
    }
    if inputs_to_fund.len() > 0 {
        panic!("You need to fund {} addresses first.", inputs_to_fund.len());
    }
}
