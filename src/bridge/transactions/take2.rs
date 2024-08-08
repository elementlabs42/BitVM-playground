use bitcoin::{
    absolute, Amount, EcdsaSighashType, Network, PublicKey, ScriptBuf, Transaction, TxOut,
};
use musig2::{PartialSignature, PubNonce, SecNonce};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    super::{
        connectors::{
            connector::*, connector_0::Connector0, connector_2::Connector2, connector_3::Connector3,
        },
        contexts::{operator::OperatorContext, verifier::VerifierContext},
        graphs::base::FEE_AMOUNT,
        scripts::*,
    },
    base::*,
    pre_signed::*,
    pre_signed_musig2::*,
};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Take2Transaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<ScriptBuf>,

    musig2_nonces: HashMap<usize, HashMap<PublicKey, PubNonce>>,
    musig2_signatures: HashMap<usize, HashMap<PublicKey, PartialSignature>>,
}

impl PreSignedTransaction for Take2Transaction {
    fn tx(&self) -> &Transaction { &self.tx }

    fn tx_mut(&mut self) -> &mut Transaction { &mut self.tx }

    fn prev_outs(&self) -> &Vec<TxOut> { &self.prev_outs }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> { &self.prev_scripts }
}

impl PreSignedMusig2Transaction for Take2Transaction {
    fn musig2_nonces(&self) -> &HashMap<usize, HashMap<PublicKey, PubNonce>> { &self.musig2_nonces }
    fn musig2_nonces_mut(&mut self) -> &mut HashMap<usize, HashMap<PublicKey, PubNonce>> {
        &mut self.musig2_nonces
    }
    fn musig2_signatures(&self) -> &HashMap<usize, HashMap<PublicKey, PartialSignature>> {
        &self.musig2_signatures
    }
    fn musig2_signatures_mut(
        &mut self,
    ) -> &mut HashMap<usize, HashMap<PublicKey, PartialSignature>> {
        &mut self.musig2_signatures
    }
}

impl Take2Transaction {
    pub fn new(context: &OperatorContext, input0: Input, input1: Input, input2: Input) -> Self {
        let mut this = Self::new_for_validation(
            context.network,
            &context.operator_public_key,
            &context.n_of_n_public_key,
            input0,
            input1,
            input2,
        );

        this.sign_input1(context);

        this
    }

    pub fn new_for_validation(
        network: Network,
        operator_public_key: &PublicKey,
        n_of_n_public_key: &PublicKey,
        input0: Input,
        input1: Input,
        input2: Input,
    ) -> Self {
        let connector_0 = Connector0::new(network, n_of_n_public_key);
        let connector_2 = Connector2::new(network, operator_public_key);
        let connector_3 = Connector3::new(network, n_of_n_public_key);

        let _input0 = connector_0.generate_tx_in(&input0);

        let _input1 = connector_2.generate_tx_in(&input1);

        let _input2 = connector_3.generate_tx_in(&input2);

        let total_output_amount =
            input0.amount + input1.amount + input2.amount - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: total_output_amount,
            script_pubkey: generate_pay_to_pubkey_script_address(network, operator_public_key)
                .script_pubkey(),
        };

        Take2Transaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0, _input1, _input2],
                output: vec![_output0],
            },
            prev_outs: vec![
                TxOut {
                    value: input0.amount,
                    script_pubkey: connector_0.generate_address().script_pubkey(),
                },
                TxOut {
                    value: input1.amount,
                    script_pubkey: connector_2.generate_address().script_pubkey(),
                },
                TxOut {
                    value: input2.amount,
                    script_pubkey: connector_3.generate_address().script_pubkey(),
                },
            ],
            prev_scripts: vec![
                connector_0.generate_script(),
                connector_2.generate_script(),
                connector_3.generate_script(),
            ],
            musig2_nonces: HashMap::new(),
            musig2_signatures: HashMap::new(),
        }
    }

    fn sign_input0(&mut self, context: &VerifierContext, secret_nonce: &SecNonce) {
        // pre_sign_p2wsh_input(
        //     self,
        //     context,
        //     0,
        //     EcdsaSighashType::All,
        //     &vec![&context.n_of_n_keypair],
        // );
    }

    fn sign_input1(&mut self, context: &OperatorContext) {
        pre_sign_p2wsh_input(
            self,
            context,
            1,
            EcdsaSighashType::All,
            &vec![&context.operator_keypair],
        );
    }

    fn sign_input2(&mut self, context: &VerifierContext, secret_nonce: &SecNonce) {
        // pre_sign_p2wsh_input(
        //     self,
        //     context,
        //     2,
        //     EcdsaSighashType::All,
        //     &vec![&context.n_of_n_keypair],
        // );
    }

    pub fn push_nonces(&mut self, context: &VerifierContext) -> HashMap<usize, SecNonce> {
        let mut secret_nonces = HashMap::new();

        let input_index = 0;
        let secret_nonce = push_nonce(self, context, input_index);
        secret_nonces.insert(input_index, secret_nonce);

        let input_index = 2;
        let secret_nonce = push_nonce(self, context, input_index);
        secret_nonces.insert(input_index, secret_nonce);

        secret_nonces
    }

    pub fn pre_sign(
        &mut self,
        context: &VerifierContext,
        secret_nonces: &HashMap<usize, SecNonce>,
    ) {
        self.sign_input0(context, &secret_nonces[&0]);
        self.sign_input2(context, &secret_nonces[&2]);
    }

    pub fn merge(&mut self, take2: &Take2Transaction) {
        merge_transactions(&mut self.tx, &take2.tx);
        merge_musig2_nonces_and_signatures(self, take2);
    }
}

impl BaseTransaction for Take2Transaction {
    fn finalize(&self) -> Transaction { self.tx.clone() }
}
