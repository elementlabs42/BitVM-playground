// TODO
// leaf[0] -> 2 week checksequenceverify to refund tokens to depositor
// leaf[1] -> input to peg in with inscribed ethereum address for destination of wrapped bitcoin

use bitcoin::{
  key::Secp256k1, taproot::TaprootBuilder, Address, Network, XOnlyPublicKey
};

use crate::bridge::{context::BridgeContext, scripts::{EthAddress, ScriptBuilder}};

pub struct ConnectorZ {
  depositor_public_key: XOnlyPublicKey,
  script_builder: ScriptBuilder
}

impl ConnectorZ {
    pub fn new(
      context: &BridgeContext,
      verifier_public_keys: &Vec<XOnlyPublicKey>,
      operator_public_key: &XOnlyPublicKey,
      depositor_public_key: &XOnlyPublicKey,
    ) -> Self {
      // TODO: shuld use `BridgeContext`
      // let verifier_public_keys = context
      //   .n_of_n_pubkey
      //   .expect("n_of_n_pubkey is required in context");

      let script_builder = ScriptBuilder::new(operator_public_key, verifier_public_keys);

      ConnectorZ {
        depositor_public_key: depositor_public_key.clone(),
        script_builder: script_builder
      }
    }


    pub fn connector_z_address(&self, evm_address: EthAddress) -> Address {
      let secp = Secp256k1::new();
      let timelock_script = ScriptBuilder::create_timelock_script(&self.depositor_public_key, 10); // TODO: remove hardcoded `10` value
      let peg_in_script = self.script_builder.create_peg_in_script(&self.depositor_public_key, &evm_address);

      // TODO
      let taptree = TaprootBuilder::new()
      .add_leaf(1, timelock_script)
      .expect("Unable to add timelock script")
      .add_leaf(1, peg_in_script)
      .expect("Unable to add peg in script")
      .finalize(&secp, self.depositor_public_key.clone())
      .expect("Unable to finalize deposit transaction connector Z taproot");

      Address::p2tr_tweaked(
        taptree.output_key(), 
        Network::Testnet
      )
    }
}
