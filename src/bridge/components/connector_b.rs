use crate::treepp::*;
use bitcoin::{
    hashes::{ripemd160, Hash},
    key::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Address, Network,
    XOnlyPublicKey,
};

use super::helper::*;

pub type LockScript = fn(u32) -> Script;

pub type UnlockWitness = fn(u32) -> Vec<Vec<u8>>;

pub struct KickoffLeaf {
  pub lock: LockScript,
  pub unlock: UnlockWitness,
}

pub fn kickoff_leaf() -> KickoffLeaf {
  KickoffLeaf {
    lock: |index| {
        script! {
            // TODO: Operator_key?
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

pub fn generate_kickoff_leaves() -> Vec<Script> {
  // TODO: Scripts with n_of_n_pubkey and one of the commitments disprove leaves in each leaf (Winternitz signatures)
  let mut leaves = Vec::with_capacity(1000);
  let locking_template = kickoff_leaf().lock;
  for i in 0..1000 {
    leaves.push(locking_template(i));
  }
  leaves
}

// Returns the TaprootSpendInfo for the Commitment Taptree and the corresponding pre_sign_output
pub fn connector_b_spend_info(
  n_of_n_pubkey: XOnlyPublicKey,
) -> (TaprootSpendInfo, TaprootSpendInfo) {
  let secp = Secp256k1::new();

  let scripts = generate_kickoff_leaves();
  let script_weights = scripts.iter().map(|script| (1, script.clone()));
  let commitment_taptree_info = TaprootBuilder::with_huffman_tree(script_weights)
      .expect("Unable to add kickoff leaves")
      // Finalizing with n_of_n_pubkey allows the key-path spend with the
      // n_of_n
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize kickoff transaction connector b taproot");
  let pre_sign_info = TaprootBuilder::new()
      .add_leaf(0, generate_pre_sign_script(n_of_n_pubkey))
      .expect("Unable to add pre_sign script as leaf")
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize OP_CHECKSIG taproot");
  (pre_sign_info, commitment_taptree_info)
}

pub fn connector_b_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_b_spend_info(n_of_n_pubkey).1.output_key(),
      Network::Testnet,
  )
}

pub fn connector_b_pre_sign_address(n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_b_spend_info(n_of_n_pubkey).0.output_key(),
      Network::Testnet,
  )
}