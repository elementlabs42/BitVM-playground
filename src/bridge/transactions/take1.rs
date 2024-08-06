use bitcoin::{
    absolute, Amount, EcdsaSighashType, Network, PublicKey, ScriptBuf, TapSighashType, Transaction,
    TxOut, XOnlyPublicKey,
};
use musig2::{PartialSignature, PubNonce, SecNonce};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    super::{
        connectors::{
            connector::*, connector_0::Connector0, connector_1::Connector1,
            connector_a::ConnectorA, connector_b::ConnectorB,
        },
        contexts::{operator::OperatorContext, verifier::VerifierContext},
        graphs::base::FEE_AMOUNT,
        scripts::*,
    },
    base::*,
    pre_signed::*,
    signing_musig2::{generate_nonce, get_aggregated_nonce, get_partial_signature},
};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Take1Transaction {
    tx: Transaction,
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<ScriptBuf>,
    connector_a: ConnectorA,
    connector_b: ConnectorB,

    musig2_nonces: HashMap<usize, HashMap<PublicKey, PubNonce>>,
    musig2_signatures: HashMap<usize, HashMap<PublicKey, PartialSignature>>,
}

impl PreSignedTransaction for Take1Transaction {
    fn tx(&self) -> &Transaction { &self.tx }

    fn tx_mut(&mut self) -> &mut Transaction { &mut self.tx }

    fn prev_outs(&self) -> &Vec<TxOut> { &self.prev_outs }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> { &self.prev_scripts }
}

impl Take1Transaction {
    pub fn new(
        context: &OperatorContext,
        input0: Input,
        input1: Input,
        input2: Input,
        input3: Input,
    ) -> Self {
        let mut this = Self::new_for_validation(
            context.network,
            &context.operator_public_key,
            &context.operator_taproot_public_key,
            &context.n_of_n_public_key,
            &context.n_of_n_taproot_public_key,
            input0,
            input1,
            input2,
            input3,
        );

        this.sign_input1(context);
        this.sign_input2(context);

        this
    }

    pub fn new_for_validation(
        network: Network,
        operator_public_key: &PublicKey,
        operator_taproot_public_key: &XOnlyPublicKey,
        n_of_n_public_key: &PublicKey,
        n_of_n_taproot_public_key: &XOnlyPublicKey,
        input0: Input,
        input1: Input,
        input2: Input,
        input3: Input,
    ) -> Self {
        let connector_0 = Connector0::new(network, n_of_n_public_key);
        let connector_1 = Connector1::new(network, operator_public_key);
        let connector_a = ConnectorA::new(
            network,
            operator_taproot_public_key,
            n_of_n_taproot_public_key,
        );
        let connector_b = ConnectorB::new(network, n_of_n_taproot_public_key);

        let _input0 = connector_0.generate_tx_in(&input0);

        let _input1 = connector_1.generate_tx_in(&input1);

        let _input2 = connector_a.generate_taproot_leaf_tx_in(0, &input2);

        let _input3 = connector_b.generate_taproot_leaf_tx_in(0, &input3);

        let total_output_amount = input0.amount + input1.amount + input2.amount + input3.amount
            - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: total_output_amount,
            script_pubkey: generate_pay_to_pubkey_script_address(network, operator_public_key)
                .script_pubkey(),
        };

        Take1Transaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0, _input1, _input2, _input3],
                output: vec![_output0],
            },
            prev_outs: vec![
                TxOut {
                    value: input0.amount,
                    script_pubkey: connector_0.generate_address().script_pubkey(),
                },
                TxOut {
                    value: input1.amount,
                    script_pubkey: connector_1.generate_address().script_pubkey(),
                },
                TxOut {
                    value: input2.amount,
                    script_pubkey: connector_a.generate_taproot_address().script_pubkey(),
                },
                TxOut {
                    value: input3.amount,
                    script_pubkey: connector_b.generate_taproot_address().script_pubkey(),
                },
            ],
            prev_scripts: vec![
                connector_0.generate_script(),
                connector_1.generate_script(),
                connector_a.generate_taproot_leaf_script(0),
                connector_b.generate_taproot_leaf_script(0),
            ],
            connector_a,
            connector_b,
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

    fn sign_input2(&mut self, context: &OperatorContext) {
        pre_sign_taproot_input(
            self,
            context,
            2,
            TapSighashType::All,
            self.connector_a.generate_taproot_spend_info(),
            &vec![&context.operator_keypair],
        );
    }

    fn sign_input3(&mut self, context: &VerifierContext, secret_nonce: &SecNonce) {
        // pre_sign_taproot_input(
        //     self,
        //     context,
        //     3,
        //     TapSighashType::All,
        //     self.connector_b.generate_taproot_spend_info(),
        //     &vec![&context.n_of_n_keypair],
        // );

        // TODO validate nonces first

        let input_index = 3;
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

        let input_index = 3;
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
        self.sign_input3(context, &secret_nonces[&3]);
    }

    pub fn merge(&mut self, take1: &Take1Transaction) {
        merge_transactions(&mut self.tx, &take1.tx);
    }
}

impl BaseTransaction for Take1Transaction {
    fn finalize(&self) -> Transaction { self.tx.clone() }
}
