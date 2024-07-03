use crate::treepp::*;
use bitcoin::{absolute, consensus, Amount, ScriptBuf, TapSighashType, Transaction, TxOut};
use serde::{Deserialize, Serialize};

use super::{
    super::{
        connectors::{connector::*, connector_z::ConnectorZ},
        contexts::depositor::DepositorContext,
        graphs::base::FEE_AMOUNT,
        scripts::*,
    },
    base::*,
    pre_signed::*,
};

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct PegInRefundTransaction {
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    tx: Transaction,
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<Script>,
    connector_z: ConnectorZ,
}

impl PreSignedTransaction for PegInRefundTransaction {
    fn tx(&self) -> &Transaction { &self.tx }

    fn tx_mut(&mut self) -> &mut Transaction { &mut self.tx }

    fn prev_outs(&self) -> &Vec<TxOut> { &self.prev_outs }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> { &self.prev_scripts }
}

impl PegInRefundTransaction {
    pub fn new(context: &DepositorContext, evm_address: &str, input0: Input) -> Self {
        let connector_z = ConnectorZ::new(
            context.network,
            evm_address,
            &context.depositor_taproot_public_key,
            &context.n_of_n_taproot_public_key,
        );

        let _input0 = connector_z.generate_taproot_leaf_tx_in(0, &input0);

        let total_output_amount = input0.amount - Amount::from_sat(FEE_AMOUNT);

        let _output0 = TxOut {
            value: total_output_amount,
            script_pubkey: generate_pay_to_pubkey_script_address(
                context.network,
                &context.depositor_public_key,
            )
            .script_pubkey(),
        };

        let mut this = PegInRefundTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input0],
                output: vec![_output0],
            },
            prev_outs: vec![TxOut {
                value: input0.amount,
                script_pubkey: connector_z.generate_taproot_address().script_pubkey(),
            }],
            prev_scripts: vec![connector_z.generate_taproot_leaf_script(0)],
            connector_z,
        };

        this.sign_input0(context);

        this
    }

    fn sign_input0(&mut self, context: &DepositorContext) {
        pre_sign_taproot_input(
            self,
            context,
            0,
            TapSighashType::All,
            self.connector_z.generate_taproot_spend_info(),
            &vec![&context.depositor_keypair],
        );
    }
}

impl BaseTransaction for PegInRefundTransaction {
    fn finalize(&self) -> Transaction { self.tx.clone() }
}
