use crate::treepp::*;
use bitcoin::{
  hashes::{ripemd160, Hash},
  XOnlyPublicKey,
  ScriptBuf
};

pub fn generate_pre_sign_script(n_of_n_pubkey: XOnlyPublicKey) -> Script {
  script! {
      { n_of_n_pubkey }
      OP_CHECKSIG
  }
}

pub fn generate_n_of_n_script(
  operator_pubkey: XOnlyPublicKey,
  n_of_n_pubkey: XOnlyPublicKey,
) -> ScriptBuf {
  script! {
    { operator_pubkey }
    OP_CHECKSIGVERIFY
    { n_of_n_pubkey }
    OP_CHECKSIGVERIFY
  }
}

pub fn generate_commit_script(
  operator_pubkey: XOnlyPublicKey,
  n_of_n_pubkey: XOnlyPublicKey,
) -> ScriptBuf {
  script! {
    // TODO commit to intermediate values
    { operator_pubkey }
    OP_CHECKSIGVERIFY
    { n_of_n_pubkey }
    OP_CHECKSIGVERIFY
  }
}

pub fn forge_preimage(index: u32) -> String {
  format!("SECRET_{}", index)
}

pub fn all_preimages() -> Vec<Vec<u8>> {
  (0..1000).map(|i| forge_preimage(i).as_bytes().to_vec()).collect()
}

// Specialized for assert leaves currently.a
// TODO: Attach the pubkeys after constructing leaf scripts
pub type LockScript = fn(u32, XOnlyPublicKey) -> Script;

pub type UnlockWitness = fn(u32) -> Vec<Vec<u8>>;

pub struct AssertLeaf {
    pub lock: LockScript,
    pub unlock: UnlockWitness,
}

pub fn assert_leaf() -> AssertLeaf {
  AssertLeaf {
      lock: |index, operator_pubkey: XOnlyPublicKey| {
          script! {
              { operator_pubkey }
              OP_CHECKSIGVERIFY
              OP_RIPEMD160
              { ripemd160::Hash::hash(forge_preimage(index).as_bytes()).as_byte_array().to_vec() }
              OP_EQUALVERIFY
              { index }
              OP_DROP
              OP_TRUE
          }
      },
      unlock: |index| vec![forge_preimage(index).as_bytes().to_vec()],
  }
}

pub fn generate_assert_leaves(operator_pubkey: XOnlyPublicKey) -> Vec<Script> {
  // TODO: Scripts with n_of_n_pubkey and one of the commitments disprove leaves in each leaf (Winternitz signatures)
  let mut leaves = Vec::with_capacity(1000);
  let locking_template = assert_leaf().lock;
  for i in 0..1000 {
      leaves.push(locking_template(i, operator_pubkey));
  }
  leaves
}

pub fn operator_timelock_script(operator_pubkey: XOnlyPublicKey, num_of_weeks: i64) -> ScriptBuf {
  let expected_height = NUM_BLOCKS_PER_WEEK * num_of_weeks;
  script! {
    { expected_height }
    OP_CSV
    OP_DROP
    { operator_pubkey }
    OP_CHECKSIGVERIFY
  }
}

pub const NUM_BLOCKS_PER_WEEK: i64 = 1008;
