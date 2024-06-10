use crate::treepp::*;
use bitcoin::{
    absolute, key::Keypair, secp256k1::Message, sighash::{Prevouts, SighashCache}, taproot::LeafVersion, Address, Amount, Network, OutPoint, Sequence, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Witness
};

use super::super::context::BridgeContext;
use super::super::graph::{FEE_AMOUNT, N_OF_N_SECRET};

use super::bridge::*;
use super::connector_c::*;
use super::helper::*;
pub struct DisproveTransaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    script_index: u32,
}

impl DisproveTransaction {
    pub fn new(
        context: &BridgeContext,
        pre_sign_input: Input,
        connector_c_input: Input,
        script_index: u32,
    ) -> Self {
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        let _input0 = TxIn {
            previous_output: pre_sign_input.0,
            script_sig: Script::new(), // Question: Why is this empty? IS it because it's using segwit?
            sequence: Sequence::MAX,
            witness: Witness::default(), // Question: This gets filled in during pre-sign and finalize later
        };

        let _input1 = TxIn {
            previous_output: connector_c_input.0,
            script_sig: Script::new(), // Question: Why is this empty? IS it because it's using segwit?
            sequence: Sequence::MAX,
            witness: Witness::default(), // Question: This gets filled in during pre-sign and finalize later
        };

        let total_input_amount = pre_sign_input.1 + connector_c_input.1 - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: total_input_amount / 2,
            script_pubkey:  Address::p2sh(
                &generate_burn_script(),
                Network::Testnet,
            )
            .expect("Unable to generate output script")
            .script_pubkey(),
        };

        DisproveTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0, _input1],
                output: vec![_output0],
            },
            prev_outs: vec![
                TxOut {
                    value: pre_sign_input.1,
                    script_pubkey: connector_c_pre_sign_address(n_of_n_pubkey).script_pubkey(),
                },
                TxOut {
                    value: connector_c_input.1,
                    script_pubkey: connector_c_address(n_of_n_pubkey).script_pubkey(),
                },
            ],
            script_index,
        }
    }
}

impl BridgeTransaction for DisproveTransaction {
    //TODO: Real presign
    fn pre_sign(&mut self, context: &BridgeContext) {
        let n_of_n_key = Keypair::from_seckey_str(&context.secp, N_OF_N_SECRET).unwrap();
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        // Create the signature with n_of_n_key as part of the setup
        let prevouts = Prevouts::All(&self.prev_outs);
        let prevout_leaf = ( // Question: Why can't we read this from the connector instead of redefining it here?
            generate_pre_sign_script(n_of_n_pubkey),
            LeafVersion::TapScript,
        );

        // Use Single to sign only the burn output with the n_of_n_key
        let sighash_type = TapSighashType::Single;
        let leaf_hash =
            TapLeafHash::from_script(prevout_leaf.0.clone().as_script(), LeafVersion::TapScript);
        let mut sighash_cache = SighashCache::new(&self.tx);
        let sighash = sighash_cache
            .taproot_script_spend_signature_hash(0, &prevouts, leaf_hash, sighash_type)
            .expect("Failed to construct sighash");

        let msg = Message::from(sighash);
        let signature = context.secp.sign_schnorr_no_aux_rand(&msg, &n_of_n_key);

        let signature_with_type = bitcoin::taproot::Signature {
            signature,
            sighash_type,
        };

        // Fill in the pre_sign/checksig input's witness
        let spend_info = connector_c_spend_info(n_of_n_pubkey).0;
        let control_block = spend_info
            .control_block(&prevout_leaf)
            .expect("Unable to create Control block");
        self.tx.input[0].witness.push(signature_with_type.to_vec());
        self.tx.input[0].witness.push(prevout_leaf.0.to_bytes());
        self.tx.input[0].witness.push(control_block.serialize());
    }

    fn finalize(&self, context: &BridgeContext) -> Transaction {
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        let prevout_leaf = (
            (assert_leaf().lock)(n_of_n_pubkey, self.script_index),
            LeafVersion::TapScript,
        );
        let spend_info = connector_c_spend_info(n_of_n_pubkey).1;
        let control_block = spend_info
            .control_block(&prevout_leaf)
            .expect("Unable to create Control block");

        // Push the unlocking values, script and control_block onto the witness.
        let mut tx = self.tx.clone();
        // Unlocking script
        let mut witness_vec = (assert_leaf().unlock)(self.script_index);
        // Script and Control block
        witness_vec.extend_from_slice(&[prevout_leaf.0.to_bytes(), control_block.serialize()]);

        tx.input[1].witness = Witness::from(witness_vec);
        tx
    }
}