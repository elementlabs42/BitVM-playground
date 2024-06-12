use crate::{bridge::utils::{constants::NUM_BLOCKS_PER_WEEK, scripts::generate_timelock_script}, treepp::*};
use bitcoin::{
    hashes::{ripemd160, Hash}, key::Secp256k1, taproot::{TaprootBuilder, TaprootSpendInfo}, Address, Network, XOnlyPublicKey
};

use crate::bridge::utils::{scripts::generate_pre_sign_script, types::{LockScript, UnlockWitness}};

pub struct KickOfftLeaf {
  pub lock: LockScript,
  pub unlock: UnlockWitness,
}

pub fn kick_off_leaf() -> KickOfftLeaf {
  KickOfftLeaf {
    lock: |index| {
        script! {
            // TODO: n_to_n_key?
            OP_RIPEMD160
            { ripemd160::Hash::hash(format!("SECRET_{}", index).as_bytes()).as_byte_array().to_vec() }
            OP_EQUALVERIFY
            { index }
            OP_DROP
            OP_TRUE
        }
    },
    unlock: |index| vec![format!("SECRET_{}", index).as_bytes().to_vec()],
}
}

pub fn generate_kick_off_leaves() -> Vec<Script> {
  // TODO: Scripts with n_of_n_pubkey and one of the commitments disprove leaves in each leaf (Winternitz signatures)
  let mut leaves = Vec::with_capacity(1000);
  let locking_template = kick_off_leaf().lock;
  for i in 0..1000 {
      leaves.push(locking_template(i));
  }
  leaves
}

pub fn connector_b_spend_info(n_of_n_pubkey: XOnlyPublicKey) -> TaprootSpendInfo {
  let secp = Secp256k1::new();

  TaprootBuilder::new()
      .add_leaf(1, generate_pre_sign_script(n_of_n_pubkey))
      .expect("Unable to add pre_sign script as leaf")
      .add_leaf(1, generate_timelock_script(&n_of_n_pubkey, 4*NUM_BLOCKS_PER_WEEK))
      .expect("Unable to add timelock script as leaf")
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize OP_CHECKSIG taproot")
}

pub fn connector_b_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_b_spend_info(n_of_n_pubkey).output_key(),
      Network::Testnet,
  )
}

pub fn connector_b_pre_sign_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_b_spend_info(n_of_n_pubkey).output_key(),
      Network::Testnet,
  )
}