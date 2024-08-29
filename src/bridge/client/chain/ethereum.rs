use std::str::FromStr;

use alloy::rpc::types::Log;

use super::{base::ChainAdaptor, chain::PegInEvent, chain::PegOutEvent};
use alloy::sol_types::SolEvent;
use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address as EvmAddress,
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::Filter,
    sol,
    transports::http::{reqwest::Url, Client, Http},
};
use async_trait::async_trait;
use bitcoin::hashes::Hash;
use bitcoin::{Address, Amount, Denomination, OutPoint, PublicKey, Txid};
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
            bytes operator_pubKey
        );
        event PegInMinted(
            address indexed depositor,
            uint256 amount,
            bytes32 depositorPubKey
        );
    }
);

pub struct EthereumAdaptor {
    bridge_address: EvmAddress,
    bridge_creation_block: u64,
    provider: RootProvider<Http<Client>>,
}

#[async_trait]
impl ChainAdaptor for EthereumAdaptor {
    async fn get_peg_out_init_event(&self) -> Result<Vec<PegOutEvent>, String> {
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Number(self.bridge_creation_block))
            .address(self.bridge_address)
            .event(&IBridge::PegOutInitiated::SIGNATURE);

        let results = self.provider.get_logs(&filter).await;
        if results.is_err() {
            return Err(results.unwrap_err().to_string());
        }
        let logs = results.unwrap();
        let mut sol_events: Vec<Log<IBridge::PegOutInitiated>> = Vec::new();
        for log in logs {
            let decoded = log.log_decode::<IBridge::PegOutInitiated>();
            if decoded.is_err() {
                return Err(decoded.unwrap_err().to_string());
            }
            sol_events.push(decoded.unwrap());
        }

        let peg_out_events = sol_events
            .iter()
            .map(|e| {
                let withdrawer_address = Address::from_str(&e.inner.data.destination_address)
                    .unwrap()
                    .assume_checked();
                let operator_public_key =
                    PublicKey::from_slice(&e.inner.data.operator_pubKey.to_vec()).unwrap();
                PegOutEvent {
                    withdrawer_chain_address: e.inner.data.withdrawer.to_string(),
                    withdrawer_public_key_hash: withdrawer_address.pubkey_hash().unwrap(),
                    source_outpoint: OutPoint {
                        txid: Txid::from_slice(&e.inner.data.source_outpoint.txId.to_vec())
                            .unwrap(),
                        vout: e.inner.data.source_outpoint.vOut.to::<u32>(),
                    },
                    amount: Amount::from_str_in(
                        e.inner.data.amount.to_string().as_str(),
                        Denomination::Satoshi,
                    )
                    .unwrap(),
                    operator_public_key,
                    timestamp: u32::try_from(e.block_timestamp.unwrap()).unwrap(),
                }
            })
            .collect();

        Ok(peg_out_events)
    }

    async fn get_peg_in_minted_event(&self) -> Result<Vec<PegInEvent>, String> {
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Number(self.bridge_creation_block))
            .address(self.bridge_address)
            .event(&IBridge::PegInMinted::SIGNATURE);

        let results = self.provider.get_logs(&filter).await;
        if results.is_err() {
            return Err(results.unwrap_err().to_string());
        }
        let logs = results.unwrap();
        let mut sol_events: Vec<IBridge::PegInMinted> = Vec::new();
        // parse from sol_events to pegin minted events
        for log in logs {
            let decoded = log.log_decode::<IBridge::PegInMinted>();
            if decoded.is_err() {
                return Err(decoded.unwrap_err().to_string());
            }
            sol_events.push(decoded.unwrap().inner.data);
        }

        let peg_in_minted_events = sol_events
            .iter()
            .map(|e| PegInEvent {
                depositor: e.depositor.to_string(),
                amount: Amount::from_str_in(e.amount.to_string().as_str(), Denomination::Satoshi)
                    .unwrap(),
                depositor_pubkey: PublicKey::from_slice(&e.depositorPubKey.to_vec()).unwrap(),
            })
            .collect();

        Ok(peg_in_minted_events)
    }
}

impl EthereumAdaptor {
    pub fn new() -> Option<Self> {
        dotenv::dotenv().ok();
        let rpc_url_str = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_RPC_URL");
        let bridge_address_str = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_BRIDGE_ADDRESS");
        let bridge_creation = dotenv::var("BRIDGE_CHAIN_ADAPTOR_ETHEREUM_BRIDGE_CREATION");
        if bridge_address_str.is_err() || bridge_creation.is_err() {
            return None;
        }
        if rpc_url_str.is_err() {
            return None;
        }

        let rpc_url = rpc_url_str.unwrap().parse::<Url>();
        let bridge_address = bridge_address_str.unwrap().parse::<EvmAddress>();
        Some(Self {
            bridge_address: bridge_address.unwrap(),
            bridge_creation_block: bridge_creation.unwrap().parse::<u64>().unwrap(),
            provider: ProviderBuilder::new().on_http(rpc_url.unwrap()),
        })
    }
}
