use std::collections::HashMap;

use bitcoin::{
    absolute, consensus, Amount, Network, PublicKey, ScriptBuf, TapSighashType, Transaction, TxOut,
    XOnlyPublicKey,
};
use musig2::{BinaryEncoding, PartialSignature, PubNonce, SecNonce};
use serde::{Deserialize, Serialize};

use super::{
    super::{
        connectors::{connector::*, connector_0::Connector0, connector_z::ConnectorZ},
        contexts::{depositor::DepositorContext, verifier::VerifierContext},
        graphs::base::FEE_AMOUNT,
    },
    base::*,
    pre_signed::*,
    signing::*,
    signing_musig2::{get_aggregated_nonce, get_aggregated_signature, get_partial_signature},
};

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct PegInConfirmTransaction {
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    tx: Transaction,
    pub musig2_nonces: HashMap<PublicKey, PubNonce>, // TODO: Consider changing to private and adding read-only access via the PreSignedTransaction trait
    pub musig2_signatures: HashMap<PublicKey, PartialSignature>,
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<ScriptBuf>,
    connector_z: ConnectorZ,
}

impl PreSignedTransaction for PegInConfirmTransaction {
    fn tx(&self) -> &Transaction { &self.tx }

    fn tx_mut(&mut self) -> &mut Transaction { &mut self.tx }

    fn prev_outs(&self) -> &Vec<TxOut> { &self.prev_outs }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> { &self.prev_scripts }
}

impl PegInConfirmTransaction {
    pub fn new(context: &DepositorContext, evm_address: &str, input0: Input) -> Self {
        let mut this = Self::new_for_validation(
            context.network,
            &context.taproot_public_key,
            &context.n_of_n_public_key,
            &context.n_of_n_taproot_public_key,
            evm_address,
            input0,
        );

        this.push_depositor_signature_input0(context);

        this
    }

    pub fn new_for_validation(
        network: Network,
        depositor_taproot_public_key: &XOnlyPublicKey,
        n_of_n_public_key: &PublicKey,
        n_of_n_taproot_public_key: &XOnlyPublicKey,
        evm_address: &str,
        input0: Input,
    ) -> Self {
        let connector_0 = Connector0::new(network, n_of_n_public_key);
        let connector_z = ConnectorZ::new(
            network,
            evm_address,
            depositor_taproot_public_key,
            n_of_n_taproot_public_key,
        );

        let _input0 = connector_z.generate_taproot_leaf_tx_in(1, &input0);

        let total_output_amount = input0.amount - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: total_output_amount,
            script_pubkey: connector_0.generate_address().script_pubkey(),
        };

        PegInConfirmTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0],
                output: vec![_output0],
            },
            musig2_nonces: HashMap::new(),
            musig2_signatures: HashMap::new(),
            prev_outs: vec![TxOut {
                value: input0.amount,
                script_pubkey: connector_z.generate_taproot_address().script_pubkey(),
            }],
            prev_scripts: vec![connector_z.generate_taproot_leaf_script(1)],
            connector_z,
        }
    }

    fn push_depositor_signature_input0(&mut self, context: &DepositorContext) {
        let input_index = 0;
        push_taproot_leaf_signature_to_witness(
            context,
            &mut self.tx,
            &self.prev_outs,
            input_index,
            TapSighashType::All,
            &self.prev_scripts[input_index],
            &context.keypair,
        );
    }

    // fn push_n_of_n_signature_input0(&mut self, context: &VerifierContext) {
    //     let input_index = 0;
    //     push_taproot_leaf_signature_to_witness(
    //         context,
    //         &mut self.tx,
    //         &self.prev_outs,
    //         input_index,
    //         TapSighashType::All,
    //         &self.prev_scripts[input_index],
    //         &context.n_of_n_keypair,
    //     );
    // }

    fn finalize_input0(&mut self) {
        let input_index = 0;
        push_taproot_leaf_script_and_control_block_to_witness(
            &mut self.tx,
            input_index,
            &self.connector_z.generate_taproot_spend_info(),
            &self.prev_scripts[input_index],
        );
    }

    pub fn push_nonce(&mut self, public_key: PublicKey, public_nonce: PubNonce) {
        self.musig2_nonces.insert(public_key, public_nonce);
    }

    fn push_partial_signature_input0(&mut self, context: &VerifierContext, secnonce: &SecNonce) {
        let input_index = 0;
        let partial_signature = get_partial_signature(
            context,
            &self.tx,
            secnonce,
            &get_aggregated_nonce(self.musig2_nonces.values()),
            input_index,
            &self.prev_outs,
            &self.prev_scripts[input_index],
            TapSighashType::All,
        )
        .unwrap(); // TODO: Add error handling.

        self.push_signature(context.public_key, partial_signature);
    }

    fn push_signature(&mut self, public_key: PublicKey, partial_sig: PartialSignature) {
        self.musig2_signatures.insert(public_key, partial_sig);
    }

    pub fn pre_sign(&mut self, context: &VerifierContext, secnonce: &SecNonce) {
        self.push_partial_signature_input0(context, secnonce);
    }

    /// Generate the final Schnorr signature and push it to the witness in this tx.
    // TODO: Compare with BaseTransaction::finalize() and refactor as needed.
    pub fn finalize(&mut self, context: &VerifierContext) {
        // TODO: Verify we have partial signatures from all verifiers.
        // TODO: Verify each signature against the signers public key.
        // See example here: https://github.com/conduition/musig2/blob/c39bfce58098d337a3ec38b54d93def8306d9953/src/signing.rs#L358C1-L366C65

        let input_index = 0;
        let final_signature = get_aggregated_signature(
            context,
            &self.tx,
            &get_aggregated_nonce(self.musig2_nonces.values()),
            input_index,
            &self.prev_outs,
            &self.prev_scripts[input_index],
            TapSighashType::All,
            self.musig2_signatures
                .values()
                .map(|&ps| PartialSignature::from(ps))
                .collect(), // TODO: Is there a more elegant way of doing this?
        )
        .unwrap(); // TODO: Add error handling.
        self.tx.input[input_index]
            .witness
            .push(final_signature.to_bytes());

        self.finalize_input0();

        // TODO: Consider verifying the final signature against the n-of-n public key and the tx.
    }

    pub fn merge(&mut self, peg_in_confirm: &PegInConfirmTransaction) {
        merge_transactions(&mut self.tx, &peg_in_confirm.tx);
    }
}

impl BaseTransaction for PegInConfirmTransaction {
    fn finalize(&self) -> Transaction { self.tx.clone() }
}
