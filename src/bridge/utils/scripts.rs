use crate::treepp::*;
use bitcoin::{
  script::Builder,
  opcodes::all::*,
  ScriptBuf, XOnlyPublicKey,
};

pub fn generate_pre_sign_script(pubkey: XOnlyPublicKey) -> Script {
  script! {
    { pubkey }
    OP_CHECKSIG
  }
}

pub fn generate_burn_script() -> Script {
  script! {
      OP_RETURN
  }
}

pub fn generate_timelock_script(
  depositor_public_key: &XOnlyPublicKey,
  blocks: i64
) -> ScriptBuf {
  Builder::new()
    .push_int(blocks)
    .push_opcode(OP_CSV)
    .push_opcode(OP_DROP)
    .push_x_only_key(depositor_public_key)
    .push_opcode(OP_CHECKSIG)
    .into_script()
}