use bitcoin::{
    key::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Address, Network,
    XOnlyPublicKey,
};

use crate::bridge::utils::scripts::generate_pre_sign_script;

pub fn connector_a_spend_info(n_of_n_pubkey: XOnlyPublicKey) -> TaprootSpendInfo {
  let secp = Secp256k1::new();

  TaprootBuilder::new()
      .add_leaf(0, generate_pre_sign_script(n_of_n_pubkey))
      .expect("Unable to add pre_sign script as leaf")
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize OP_CHECKSIG taproot")
}

pub fn connector_a_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_a_spend_info(n_of_n_pubkey).output_key(),
      Network::Testnet,
  )
}

pub fn connector_a_pre_sign_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_a_spend_info(n_of_n_pubkey).output_key(),
      Network::Testnet,
  )
}