use bitcoin::{
    absolute, consensus, Amount, Network, PublicKey, ScriptBuf, TapSighashType, Transaction, TxOut,
    XOnlyPublicKey,
};
use musig2::{BinaryEncoding, PartialSignature, PubNonce, SecNonce};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::bridge::contexts::base::BaseContext;

use super::{
    super::{
        connectors::{
            connector::*, connector_2::Connector2, connector_3::Connector3,
            connector_b::ConnectorB, connector_c::ConnectorC,
        },
        contexts::{operator::OperatorContext, verifier::VerifierContext},
        graphs::base::{DUST_AMOUNT, FEE_AMOUNT},
    },
    base::*,
    pre_signed::*,
    signing::push_taproot_leaf_script_and_control_block_to_witness,
    signing_musig2::{
        generate_nonce, get_aggregated_nonce, get_aggregated_signature, get_partial_signature,
    },
};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct AssertTransaction {
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    tx: Transaction,
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<ScriptBuf>,
    connector_b: ConnectorB,

    musig2_nonces: HashMap<usize, HashMap<PublicKey, PubNonce>>,
    musig2_signatures: HashMap<usize, HashMap<PublicKey, PartialSignature>>,
}

impl PreSignedTransaction for AssertTransaction {
    fn tx(&self) -> &Transaction { &self.tx }

    fn tx_mut(&mut self) -> &mut Transaction { &mut self.tx }

    fn prev_outs(&self) -> &Vec<TxOut> { &self.prev_outs }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> { &self.prev_scripts }
}

impl AssertTransaction {
    pub fn new(context: &OperatorContext, input0: Input) -> Self {
        Self::new_for_validation(
            context.network,
            &context.operator_public_key,
            &context.n_of_n_public_key,
            &context.n_of_n_taproot_public_key,
            input0,
        )
    }

    pub fn new_for_validation(
        network: Network,
        operator_public_key: &PublicKey,
        n_of_n_public_key: &PublicKey,
        n_of_n_taproot_public_key: &XOnlyPublicKey,
        input0: Input,
    ) -> Self {
        let connector_2 = Connector2::new(network, operator_public_key);
        let connector_3 = Connector3::new(network, n_of_n_public_key);
        let connector_b = ConnectorB::new(network, n_of_n_taproot_public_key);
        let connector_c = ConnectorC::new(network, n_of_n_taproot_public_key);

        let _input0 = connector_b.generate_taproot_leaf_tx_in(1, &input0);

        let total_output_amount = input0.amount - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: Amount::from_sat(DUST_AMOUNT),
            script_pubkey: connector_2.generate_address().script_pubkey(),
        };

        let _output1 = TxOut {
            value: total_output_amount - Amount::from_sat(DUST_AMOUNT) * 2,
            script_pubkey: connector_3.generate_address().script_pubkey(),
        };

        let _output2 = TxOut {
            value: Amount::from_sat(DUST_AMOUNT),
            script_pubkey: connector_c.generate_taproot_address().script_pubkey(),
        };

        AssertTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0],
                output: vec![_output0, _output1, _output2],
            },
            prev_outs: vec![TxOut {
                value: input0.amount,
                script_pubkey: connector_b.generate_taproot_address().script_pubkey(),
            }],
            prev_scripts: vec![connector_b.generate_taproot_leaf_script(1)],
            connector_b,
            musig2_nonces: HashMap::new(),
            musig2_signatures: HashMap::new(),
        }
    }

    fn sign_input0(&mut self, context: &VerifierContext, secret_nonce: &SecNonce) {
        // pre_sign_taproot_input(
        //     self,
        //     context,
        //     0,
        //     TapSighashType::All,
        //     self.connector_b.generate_taproot_spend_info(),
        //     &vec![&context.n_of_n_keypair],
        // );

        let input_index = 0;
        let partial_signature = get_partial_signature(
            context,
            &self.tx,
            secret_nonce,
            &get_aggregated_nonce(self.musig2_nonces[&input_index].values()),
            input_index,
            &self.prev_outs,
            &self.prev_scripts[input_index],
            TapSighashType::All,
        )
        .unwrap(); // TODO: Add error handling.

        if self.musig2_signatures.get(&input_index).is_none() {
            self.musig2_signatures.insert(input_index, HashMap::new());
        }
        self.musig2_signatures
            .get_mut(&input_index)
            .unwrap()
            .insert(context.verifier_public_key, partial_signature);

        // TODO: Consider verifying the final signature against the n-of-n public key and the tx.
        if self.musig2_signatures[&input_index].len() == context.n_of_n_public_keys.len() {
            self.finalize_input0(context);
        }
    }

    fn finalize_input0(&mut self, context: &dyn BaseContext) {
        // TODO: Verify we have partial signatures from all verifiers.
        // TODO: Verify each signature against the signers public key.
        // See example here: https://github.com/conduition/musig2/blob/c39bfce58098d337a3ec38b54d93def8306d9953/src/signing.rs#L358C1-L366C65

        // Aggregate + push signature
        let input_index = 0;
        let final_signature = get_aggregated_signature(
            context,
            &self.tx,
            &get_aggregated_nonce(self.musig2_nonces[&input_index].values()),
            input_index,
            &self.prev_outs,
            &self.prev_scripts[input_index],
            TapSighashType::All,
            self.musig2_signatures[&input_index]
                .values()
                .map(|&partial_signature| PartialSignature::from(partial_signature))
                .collect(), // TODO: Is there a more elegant way of doing this?
        )
        .unwrap(); // TODO: Add error handling.
        self.tx.input[input_index]
            .witness
            .push(final_signature.to_bytes());

        // Push script + control block
        push_taproot_leaf_script_and_control_block_to_witness(
            &mut self.tx,
            input_index,
            &self.connector_b.generate_taproot_spend_info(),
            &self.prev_scripts[input_index],
        );
    }

    pub fn push_nonces(&mut self, context: &VerifierContext) -> HashMap<usize, SecNonce> {
        let mut secret_nonces = HashMap::new();

        let input_index = 0;
        let secret_nonce = generate_nonce();
        if self.musig2_nonces.get(&input_index).is_none() {
            self.musig2_nonces.insert(input_index, HashMap::new());
        }
        self.musig2_nonces
            .get_mut(&input_index)
            .unwrap()
            .insert(context.verifier_public_key, secret_nonce.public_nonce());
        secret_nonces.insert(input_index, secret_nonce);

        secret_nonces
    }

    pub fn pre_sign(
        &mut self,
        context: &VerifierContext,
        secret_nonces: &HashMap<usize, SecNonce>,
    ) {
        self.sign_input0(context, &secret_nonces[&0]);
    }

    pub fn merge(&mut self, assert: &AssertTransaction) {
        merge_transactions(&mut self.tx, &assert.tx);
    }
}

impl BaseTransaction for AssertTransaction {
    fn finalize(&self) -> Transaction { self.tx.clone() }
}
