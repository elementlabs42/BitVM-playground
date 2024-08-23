use std::io::Read;

use super::{base::ChainAdaptor, chain::PegOutEvent};
use alloy::network::Ethereum;
use alloy::sol_types::SolEvent;
use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::Filter,
    sol,
    transports::http::{reqwest::Url, Client, Http},
};
use async_trait::async_trait;
use bitcoin::hashes::Hash;
use bitcoin::{OutPoint, Txid};
use dotenv;

sol!(
    #[derive(Debug)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IBridge {
        struct Outpoint {
            bytes32 txId;
            uint256 vOut;
        }
        event PegOutInitiated(
            address indexed withdrawer,
            string destination_address,
            Outpoint source_outpoint,
            uint256 amount,
            bytes32 operator_pubKey
        );
    }
);

pub struct EthereumAdaptor {
    bridge_address: String,
    bridge_creation_block: u64,
    provider: RootProvider<Http<Client>>,
}

#[async_trait]
impl ChainAdaptor for EthereumAdaptor {
    async fn get_peg_out_init_event(&self) -> Result<Vec<PegOutEvent>, String> {
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Number(self.bridge_creation_block))
            .address(self.bridge_address.parse::<Address>().unwrap())
            .event(&IBridge::PegOutInitiated::SIGNATURE);

        let logs = self.provider.get_logs(&filter).await.unwrap();
        println!("logs.length: {:?}", logs.len());
        let sol_events: Vec<IBridge::PegOutInitiated>;
        for log in logs {
            let decoded = log.log_decode::<IBridge::PegOutInitiated>();
            if decoded.is_err() {
                return Err(decoded.unwrap_err().to_string());
            }
            // let IBridge::PegOutInitiated {
            //     withdrawer,
            //     amount,
            //     destination_address,
            //     source_outpoint,
            //     operator_pubKey,
            // } = decoded.unwrap().inner.data;
            sol_events.push(decoded.unwrap().inner.data);
            // println!("log: {withdrawer:?} {amount:?} {destination_address:?} {source_outpoint:?} {operator_pubKey:?}");
        }

        sol_events.iter().map(|e| PegOutEvent {
            source_outpoint: OutPoint {
                txid: Txid::from_slice(&e.source_outpoint.txId.to_vec()).unwrap(),
                vout: e.source_outpoint.vOut.to::<u32>(),
            },
        });
    }
}

impl EthereumAdaptor {
    pub fn new() -> Option<Self> {
        dotenv::dotenv().ok();
        let rpc_url = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_RPC_URL");
        let bridge_address = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_BRIDGE_ADDRESS");
        let bridge_creation = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_BRIDGE_CREATION");
        if bridge_address.is_err() || bridge_creation.is_err() {
            return None;
        }
        if rpc_url.is_err() {
            return None;
        }

        Some(Self {
            bridge_address: bridge_address.unwrap(),
            bridge_creation_block: bridge_creation.unwrap().parse::<u64>().unwrap(),
            provider: ProviderBuilder::new().on_http(rpc_url.unwrap().parse::<Url>().unwrap()),
        })
    }
}
