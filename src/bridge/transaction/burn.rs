use crate::treepp::*;
use bitcoin::{
    absolute,
    key::Keypair,
    secp256k1::Message,
    sighash::{Prevouts, SighashCache},
    taproot::LeafVersion,
    Amount, OutPoint, Sequence, TapLeafHash, TapSighashType,
    Transaction, TxIn, TxOut, Witness,
};

use super::super::context::BridgeContext;
use super::super::graph::{FEE_AMOUNT, N_OF_N_SECRET, UNSPENDABLE_PUBKEY};

use crate::bridge::connector::connector_b::*;
use crate::bridge::transaction::bridge_transaction::BridgeTransaction;
use crate::bridge::utils::scripts::generate_pre_sign_script;
pub struct BurnTransaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    script_index: u32,
}

impl BurnTransaction {
    pub fn new(
        context: &BridgeContext,
        connector_b: OutPoint,
        pre_sign: OutPoint,
        connector_b_value: Amount,
        pre_sign_value: Amount,
        script_index: u32,
    ) -> Self {
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        let burn_output = TxOut {
            value: connector_b_value  - Amount::from_sat(FEE_AMOUNT) * 95 / 100,
            script_pubkey: generate_pre_sign_script(*UNSPENDABLE_PUBKEY), // TODO： should use op_return script for burning, but esplora does not support maxburnamount parameter
        };

        let connector_b_input = TxIn {
            previous_output: connector_b,
            script_sig: Script::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        };

        BurnTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![connector_b_input],
                output: vec![burn_output],
            },
            prev_outs: vec![
                TxOut {
                    value: connector_b_value,
                    script_pubkey: connector_b_address(n_of_n_pubkey).script_pubkey(),
                },
            ],
            script_index,
        }
    }
}

impl BridgeTransaction for BurnTransaction {
    //TODO: Real presign
    fn pre_sign(&mut self, context: &BridgeContext) {
        let n_of_n_key = Keypair::from_seckey_str(&context.secp, N_OF_N_SECRET).unwrap();
        let n_of_n_pubkey = context
            .n_of_n_pubkey
            .expect("n_of_n_pubkey required in context");

        // Create the signature with n_of_n_key as part of the setup
        let mut sighash_cache = SighashCache::new(&self.tx);
        let prevouts = Prevouts::All(&self.prev_outs);
        let prevout_leaf = (
            generate_pre_sign_script(n_of_n_pubkey),
            LeafVersion::TapScript,
        );

        // Use Single to sign only the burn output with the n_of_n_key
        let sighash_type = TapSighashType::Single;
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
        let spend_info = connector_b_spend_info(n_of_n_pubkey);
        let control_block = spend_info
            .control_block(&prevout_leaf)
            .expect("Unable to create Control block");
        self.tx.input[0].witness.push(signature_with_type.to_vec());
        self.tx.input[0].witness.push(prevout_leaf.0.to_bytes());
        self.tx.input[0].witness.push(control_block.serialize());
    }

    fn finalize(&self, _context: &BridgeContext) -> Transaction {
        // let n_of_n_pubkey = context
        //     .n_of_n_pubkey
        //     .expect("n_of_n_pubkey required in context");

        // let prevout_leaf = (
        //     (kick_off_leaf().lock)(self.script_index),
        //     LeafVersion::TapScript,
        // );
        // let spend_info = connector_b_spend_info(n_of_n_pubkey);
        // let control_block = spend_info
        //     .control_block(&prevout_leaf)
        //     .expect("Unable to create Control block");

        // // Push the unlocking values, script and control_block onto the witness.
        // let mut tx = self.tx.clone();
        // // // Unlocking script
        // let mut witness_vec = (kick_off_leaf().unlock)(self.script_index);
        // // Script and Control block
        // witness_vec.extend_from_slice(&[prevout_leaf.0.to_bytes(), control_block.serialize()]);

        // tx.input[1].witness = Witness::from(witness_vec);
        // tx
        self.tx.clone()
    }
}


#[cfg(test)]
mod tests {

    use bitcoin::{
        consensus::encode::serialize_hex, key::{Keypair, Secp256k1}, Amount, OutPoint, TxOut
    };

    use crate::bridge::client::BitVMClient;
    use crate::bridge::context::BridgeContext;
    use crate::bridge::graph::{DUST_AMOUNT, INITIAL_AMOUNT, N_OF_N_SECRET, OPERATOR_SECRET, UNSPENDABLE_PUBKEY};
    use crate::bridge::transaction::bridge_transaction::BridgeTransaction;
    use crate::bridge::connector::connector_b::*;
    use super::*;

    #[tokio::test]
    async fn test_burn_tx() {
        let secp = Secp256k1::new();
        let operator_key = Keypair::from_seckey_str(&secp, OPERATOR_SECRET).unwrap();
        let n_of_n_key = Keypair::from_seckey_str(&secp, N_OF_N_SECRET).unwrap();
        let client = BitVMClient::new();

        let funding_utxo_1 = client
            .get_initial_utxo(
                connector_b_address(n_of_n_key.x_only_public_key().0),
                Amount::from_sat(INITIAL_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_b_address(n_of_n_key.x_only_public_key().0),
                    INITIAL_AMOUNT
                );
            });
        let funding_utxo_0 = client
            .get_initial_utxo(
                connector_b_pre_sign_address(n_of_n_key.x_only_public_key().0),
                Amount::from_sat(DUST_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_b_pre_sign_address(n_of_n_key.x_only_public_key().0),
                    DUST_AMOUNT
                );
            });
        let funding_outpoint_0 = OutPoint {
            txid: funding_utxo_0.txid,
            vout: funding_utxo_0.vout,
        };
        let funding_outpoint_1 = OutPoint {
            txid: funding_utxo_1.txid,
            vout: funding_utxo_1.vout,
        };
        // let prev_tx_out_1 = TxOut {
        //     value: Amount::from_sat(INITIAL_AMOUNT),
        //     script_pubkey: connector_b_address(n_of_n_key.x_only_public_key().0).script_pubkey(),
        // };
        // let prev_tx_out_0 = TxOut {
        //     value: Amount::from_sat(DUST_AMOUNT),
        //     script_pubkey: connector_b_pre_sign_address(n_of_n_key.x_only_public_key().0)
        //         .script_pubkey(),
        // };
        let mut context = BridgeContext::new();
        context.set_operator_key(operator_key);
        context.set_n_of_n_pubkey(n_of_n_key.x_only_public_key().0);
        context.set_unspendable_pubkey(*UNSPENDABLE_PUBKEY);

        let mut burn_tx = BurnTransaction::new(
            &context,
            funding_outpoint_1,
            funding_outpoint_0,
            Amount::from_sat(INITIAL_AMOUNT),
            Amount::from_sat(DUST_AMOUNT),
            2,
        );

        burn_tx.pre_sign(&context);
        let tx = burn_tx.finalize(&context);
        println!("Script Path Spend Transaction: {:?}\n", tx);
        let result = client.esplora.broadcast(&tx).await;
        println!("Txid: {:?}", tx.compute_txid());
        println!("Broadcast result: {:?}\n", result);
        println!("Transaction hex: \n{}", serialize_hex(&tx));
        assert!(result.is_ok());
    }
}