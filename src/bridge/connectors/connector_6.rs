use std::collections::HashMap;

use crate::{
    bridge::{
        constants::{BITCOIN_TXID_LENGTH_IN_DIGITS, ETHEREUM_TXID_LENGTH_IN_DIGITS},
        transactions::{
            base::Input,
            signing_winternitz::{
                generate_winternitz_secret, winternitz_public_key_from_secret, WinternitzPublicKey,
                WinternitzSecret,
            },
        },
    },
    signatures::{
        winternitz::generate_public_key,
        winternitz_hash::{check_hash_sig, sign_hash},
    },
    treepp::script,
};
use bitcoin::{
    key::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Address, Network, ScriptBuf, TxIn, Txid, XOnlyPublicKey,
};

use serde::{Deserialize, Serialize};

use super::base::{generate_default_tx_in, BaseConnector, ConnectorId, TaprootConnector};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Connector6 {
    pub network: Network,
    pub operator_taproot_public_key: XOnlyPublicKey,
    pub winternitz_public_keys: HashMap<u8, WinternitzPublicKey>, // Leaf index -> WinternitzPublicKey
    pub evm_txid: Option<String>,
    pub peg_out_txid: Option<Txid>,
}

impl Connector6 {
    pub fn new(
        network: Network,
        operator_taproot_public_key: &XOnlyPublicKey,
    ) -> (Self, HashMap<u8, WinternitzSecret>) {
        let leaf_index = 0;
        let winternitz_secrets = HashMap::from([(leaf_index, generate_winternitz_secret())]);
        let winternitz_public_keys = winternitz_secrets
            .iter()
            .map(|(k, v)| (*k, winternitz_public_key_from_secret(&v)))
            .collect();

        (
            Connector6 {
                network,
                operator_taproot_public_key: operator_taproot_public_key.clone(),
                winternitz_public_keys,
                evm_txid: None,
                peg_out_txid: None,
            },
            winternitz_secrets,
        )
    }

    fn generate_taproot_leaf_0_script(&self) -> ScriptBuf {
        let secret_key = "b138982ce17ac813d505b5b40b665d404e9528e7"; // TODO replace with secret key for specific variable, generate and store secrets in local client
        let public_key = generate_public_key(secret_key);

        script! {
          { check_hash_sig(&public_key, ETHEREUM_TXID_LENGTH_IN_DIGITS) }
          { check_hash_sig(&public_key, BITCOIN_TXID_LENGTH_IN_DIGITS) }
          { self.operator_taproot_public_key }
          OP_CHECKSIG
        }
        .compile()
    }

    fn generate_taproot_leaf_0_tx_in(&self, input: &Input) -> TxIn { generate_default_tx_in(input) }

    pub fn generate_taproot_leaf_0_unlock(&self, txid: &str) -> Vec<Vec<u8>> {
        let secret_key = "b138982ce17ac813d505b5b40b665d404e9528e7"; // TODO replace with secret key for specific variable, generate and store secrets in local client
        let mut unlock_data: Vec<Vec<u8>> = Vec::new();
        let message = txid.as_bytes();

        // Push the message
        for byte in message.iter().rev() {
            unlock_data.push(vec![*byte]);
        }

        // Push the signature
        let witnernitz_signatures = sign_hash(secret_key, &message);
        for winternitz_signature in witnernitz_signatures.into_iter() {
            unlock_data.push(winternitz_signature.hash_bytes);
            unlock_data.push(vec![winternitz_signature.message_digit]);
        }

        unlock_data
    }
}

impl BaseConnector for Connector6 {
    fn id(&self) -> ConnectorId { ConnectorId::Connector6 }
}

impl TaprootConnector for Connector6 {
    fn generate_taproot_leaf_script(&self, leaf_index: u32) -> ScriptBuf {
        match leaf_index {
            0 => self.generate_taproot_leaf_0_script(),
            _ => panic!("Invalid leaf index."),
        }
    }

    fn generate_taproot_leaf_tx_in(&self, leaf_index: u32, input: &Input) -> TxIn {
        match leaf_index {
            0 => self.generate_taproot_leaf_0_tx_in(input),
            _ => panic!("Invalid leaf index."),
        }
    }

    fn generate_taproot_spend_info(&self) -> TaprootSpendInfo {
        TaprootBuilder::new()
            .add_leaf(0, self.generate_taproot_leaf_0_script())
            .expect("Unable to add leaf 0")
            .finalize(&Secp256k1::new(), self.operator_taproot_public_key) // TODO: should be operator key?
            .expect("Unable to finalize taproot")
    }

    fn generate_taproot_address(&self) -> Address {
        Address::p2tr_tweaked(
            self.generate_taproot_spend_info().output_key(),
            self.network,
        )
    }
}
