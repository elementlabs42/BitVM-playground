use bitcoin::{
  absolute,
  hashes::{ripemd160, Hash},
  key::{Keypair, Secp256k1},
  secp256k1::{All, Message},
  sighash::{Prevouts, SighashCache},
  taproot::{LeafVersion, TaprootBuilder, TaprootSpendInfo},
  script::Builder,
  Address, Amount, Network, OutPoint, ScriptBuf, Sequence, TapLeafHash, TapSighashType,
  Transaction, TxIn, TxOut, Witness, XOnlyPublicKey, ScriptBuf,
};

pub struct ScripBuilder {
  pub operator_public_key: XOnlyPublicKey,
  pub verifier_public_keys: Vec<XOnlyPublicKey>
}

impl ScriptBuilder {
  pub fn create_timelock_script(
    depositor_public_key: &XOnlyPublicKey,
    blocks: i64
  ) -> ScriptBuf {
    Builder::new()
      .push_int(blocks)
      .push_opcode(opcodes::OP_CSV)
      .push_opcode(opcodes::OP_DROP)
      .push_x_only_key(depositor_public_key)
      .push_opcode(opcodes::OP_CHECKSIG)
      .into_script()
  }
}