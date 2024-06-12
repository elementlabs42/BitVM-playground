use crate::treepp::*;
use bitcoin::{

    key::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Address, Network,
    XOnlyPublicKey,
    ScriptBuf,
};

use super::helper::*;

// Returns the TaprootSpendInfo for the Commitment Taptree and the corresponding pre_sign_output
pub fn connector_c_spend_info(
  operator_pubkey: XOnlyPublicKey,
  n_of_n_pubkey: XOnlyPublicKey,
) -> (TaprootSpendInfo, TaprootSpendInfo) {
  let secp = Secp256k1::new();

  // Leaf[0]: spendable by multisig of OPK and VPK[1…N]
  let take2_script = generate_n_of_n_script(operator_pubkey, n_of_n_pubkey);
  let leaf0 = TaprootBuilder::new()
    .add_leaf(0, take2_script)
    .expect("Unable to add pre_sign script as leaf")
    .finalize(&secp, n_of_n_pubkey)
    .expect("Unable to finalize OP_CHECKSIG taproot");

  // Leaf[i] for some i in 1,2,…1000: spendable by multisig of OPK and VPK[1…N]? (How do we do this?) plus the condition that f_{i}(z_{i-1})!=z_i
  let disprove_scripts = generate_assert_leaves(operator_pubkey);
  let script_weights = disprove_scripts.iter().map(|script| (1, script.clone()));
  let leaf1 = TaprootBuilder::with_huffman_tree(script_weights)
      .expect("Unable to add assert leaves")
      // Finalizing with n_of_n_pubkey allows the key-path spend with the
      // n_of_n
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize assert transaction connector c taproot");

  (leaf0, leaf1)
}

pub fn connector_c_commit_address(operator_pubkey: XOnlyPublicKey, n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_c_spend_info(operator_pubkey, n_of_n_pubkey).1.output_key(),
      Network::Testnet,
  )
}

pub fn connector_c_bounty_address(operator_pubkey: XOnlyPublicKey, n_of_n_pubkey: XOnlyPublicKey) -> Address {
  Address::p2tr_tweaked(
      connector_c_spend_info(operator_pubkey, n_of_n_pubkey).0.output_key(),
      Network::Testnet,
  )
}

pub fn connector_c_alt_spend_info(operator_pubkey: XOnlyPublicKey, n_of_n_pubkey: XOnlyPublicKey) -> TaprootSpendInfo {
  let secp = Secp256k1::new();

  // Leaf[0]: spendable by multisig of OPK and VPK[1…N]
  let leaf0 = script! {
    { operator_pubkey }
    OP_CHECKSIGVERIFY
    { n_of_n_pubkey }
    OP_CHECKSIGVERIFY
  };

  // Leaf[i] for some i in 1,2,…1000: spendable by multisig of OPK and VPK[1…N]? (How do we do this?) plus the condition that f_{i}(z_{i-1})!=z_i
  let mut scripts = generate_assert_leaves(operator_pubkey);

  scripts.insert(0, leaf0);

  let script_weights = scripts.iter().map(|script| (1, script.clone()));
  return TaprootBuilder::with_huffman_tree(script_weights)
      .expect("Unable to add assert leaves")
      // Finalizing with n_of_n_pubkey allows the key-path spend with the
      // n_of_n
      .finalize(&secp, n_of_n_pubkey)
      .expect("Unable to finalize assert transaction connector c taproot");
}