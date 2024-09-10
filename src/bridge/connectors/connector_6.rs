use crate::treepp::script;
use bitcoin::{Address, Network, PublicKey, ScriptBuf, TxIn, Txid};
use serde::{Deserialize, Serialize};

use super::{super::transactions::base::Input, connector::*};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Connector6 {
    pub network: Network,
    pub operator_public_key: PublicKey,
    pub evm_txid: Option<String>,
    pub peg_out_txid: Option<Txid>,
}

impl Connector6 {
    pub fn new(network: Network, operator_public_key: &PublicKey) -> Self {
        Connector6 {
            network,
            operator_public_key: operator_public_key.clone(),
            evm_txid: None,
            peg_out_txid: None,
        }
    }
}

impl P2wshConnector for Connector6 {
    fn generate_script(&self) -> ScriptBuf {
        script! {
          OP_FALSE
          OP_IF
          { self.evm_txid.clone().unwrap().into_bytes() }
          OP_ENDIF
          OP_FALSE
          OP_IF
          { self.peg_out_txid.clone().unwrap().to_string().into_bytes() }
          OP_ENDIF
          { self.operator_public_key }
          OP_CHECKSIG
        }
        .compile()
    }

    fn generate_address(&self) -> Address { Address::p2wsh(&self.generate_script(), self.network) }

    fn generate_tx_in(&self, input: &Input) -> TxIn { generate_default_tx_in(input) }
}
