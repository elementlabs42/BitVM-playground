
use bitcoin::{
    absolute,
    key::Keypair,
    secp256k1::Message,
    sighash::{Prevouts, SighashCache},
    taproot::LeafVersion,
    Amount, OutPoint, Sequence, TapLeafHash, TapSighashType,
    Transaction, TxIn, TxOut, Witness,
};

use crate::bridge::connector::connector_a::*;
use crate::bridge::context::BridgeContext;
use crate::{bridge::graph::FEE_AMOUNT, treepp::*};

pub struct ChallengeTransaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    script_index: u32,
}

impl ChallengeTransaction {
    pub fn new(
        context: &BridgeContext,
        connector_c: OutPoint,
        pre_sign: OutPoint,
        connector_c_value: Amount,
        pre_sign_value: Amount,
        script_index: u32,
    ) -> Self {
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");
        let unspendable_pubkey = context
            .unspendable_pubkey
            .expect("unspendable_pubkey required in context");

        let burn_output = TxOut {
            value: (connector_c_value - Amount::from_sat(FEE_AMOUNT)) / 2,
            script_pubkey: connector_a_address(unspendable_pubkey).script_pubkey(),
        };

        let connector_c_input = TxIn {
            previous_output: connector_c,
            script_sig: Script::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        };

        let pre_sign_input = TxIn {
            previous_output: pre_sign,
            script_sig: Script::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        };

        ChallengeTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![pre_sign_input, connector_c_input],
                output: vec![burn_output],
            },
            prev_outs: vec![
                TxOut {
                    value: pre_sign_value,
                    script_pubkey: connector_a_pre_sign_address(n_of_n_pubkey).script_pubkey(),
                },
                TxOut {
                    value: connector_c_value,
                    script_pubkey: connector_a_address(n_of_n_pubkey).script_pubkey(),
                },
            ],
            script_index,
        }
    }
}


#[cfg(test)]
mod tests {
}