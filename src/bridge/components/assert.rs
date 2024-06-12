use crate::bridge::components::helper::NUM_BLOCKS_PER_WEEK;
use crate::bridge::graph::{DUST_AMOUNT, FEE_AMOUNT, INITIAL_AMOUNT, N_OF_N_SECRET};
use crate::treepp::*;
use bitcoin::key::Keypair;
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::LeafVersion;
use bitcoin::{
    ScriptBuf, absolute, Address, Amount, Network, OutPoint, Sequence, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Witness, XOnlyPublicKey
};
use musig2::secp256k1::Message;

use super::super::context::BridgeContext;

use super::bridge::*;
use super::connector_b::{connector_b_address, connector_b_pre_sign_address};
use super::connector_c::{connector_c_alt_spend_info, connector_c_bounty_address, connector_c_commit_address, connector_c_spend_info};
use super::helper::{all_preimages, assert_leaf, generate_commit_script, generate_n_of_n_script, generate_pre_sign_script, operator_timelock_script};

pub struct AssertTransaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    script_index: u32,
}

impl AssertTransaction {
    pub fn new(context: &BridgeContext, input: OutPoint, input_value: Amount, pre_sign: OutPoint, pre_sign_value: Amount, script_index: u32) -> Self {
        let operator_key = context
            .operator_key
            .expect("operator_key required in context");
        let operator_pubkey = operator_key.x_only_public_key().0;
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey is required in context");

        let commit_input = TxIn {
            previous_output: input,
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

        let timeout_output = TxOut {
            value: Amount::from_sat(DUST_AMOUNT),
            script_pubkey: operator_timelock_script(operator_pubkey, 2),
        };
        let bounty_output = TxOut {
            value: input_value - Amount::from_sat(FEE_AMOUNT),
            script_pubkey: connector_c_bounty_address(operator_pubkey, n_of_n_pubkey).script_pubkey(),
        };
        let commit_output = TxOut {
            value: Amount::from_sat(DUST_AMOUNT),
            script_pubkey: connector_c_commit_address(operator_pubkey, n_of_n_pubkey).script_pubkey(),
        };
        AssertTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![pre_sign_input, commit_input],
                output: vec![timeout_output, bounty_output, commit_output],
            },
            prev_outs: vec![
                TxOut {
                    value: pre_sign_value,
                    script_pubkey: connector_b_pre_sign_address(operator_key.x_only_public_key().0, n_of_n_pubkey).script_pubkey(),
                },
                TxOut {
                    value: input_value,
                    script_pubkey: connector_b_address(operator_key.x_only_public_key().0, n_of_n_pubkey).script_pubkey(),
                },                
            ],
            script_index,
        }
    }
}

impl BridgeTransaction for AssertTransaction {
    fn pre_sign(&mut self, context: &BridgeContext) {
        let operator_key = context
            .operator_key
            .expect("operator_key required in context");
        let operator_pubkey = operator_key.x_only_public_key().0;
        let n_of_n_key = Keypair::from_seckey_str(&context.secp, N_OF_N_SECRET).unwrap();
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        // Create the signature with n_of_n_key as part of the setup
        let mut sighash_cache = SighashCache::new(&self.tx);
        let prevouts = Prevouts::All(&self.prev_outs);
        let prevout_leaf = (
            generate_n_of_n_script(operator_pubkey, n_of_n_pubkey),
            // generate_pre_sign_script(n_of_n_pubkey),
            LeafVersion::TapScript,
        );

        let sighash_type = TapSighashType::All;
        let leaf_hash =
            TapLeafHash::from_script(prevout_leaf.0.clone().as_script(), LeafVersion::TapScript);
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
        let spend_info = connector_c_spend_info(operator_key.x_only_public_key().0, n_of_n_pubkey).0;
        let control_block = spend_info
            .control_block(&prevout_leaf)
            .expect("Unable to create Control block");
        self.tx.input[0].witness.push(signature_with_type.to_vec());
        self.tx.input[0].witness.push(prevout_leaf.0.to_bytes());
        self.tx.input[0].witness.push(control_block.serialize());
    }

    fn finalize(&self, context: &BridgeContext) -> Transaction {
        let operator_key = context
            .operator_key
            .expect("operator_key required in context");
        let operator_pubkey = operator_key.x_only_public_key().0;
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        let prevout_leaf = (
            // generate_commit_script(operator_pubkey, n_of_n_pubkey),
            (assert_leaf().lock)(self.script_index, operator_key.x_only_public_key().0),
            LeafVersion::TapScript,
        );
        let spend_info = connector_c_spend_info(operator_key.x_only_public_key().0, n_of_n_pubkey).1;
        let control_block = spend_info
            .control_block(&prevout_leaf)
            .expect("Unable to create Control block");

        let mut tx = self.tx.clone();
        let mut witness_vec = all_preimages();
        // Script and Control block
        witness_vec.extend_from_slice(&[prevout_leaf.0.to_bytes(), control_block.serialize()]);

        tx.input[1].witness = Witness::from(witness_vec);
        tx        
     }
}
