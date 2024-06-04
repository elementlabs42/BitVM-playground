use bitcoin::{
  script::Builder,
  opcodes::{all::*, OP_FALSE, OP_TRUE},
  ScriptBuf, XOnlyPublicKey,
};

pub type EthAddress = [u8; 20];

pub struct ScriptBuilder {
  pub operator_public_key: XOnlyPublicKey,
  pub verifier_public_keys: Vec<XOnlyPublicKey>
}

impl ScriptBuilder {
  pub fn new(
    operator_public_key: &XOnlyPublicKey,
    verifier_public_keys: &Vec<XOnlyPublicKey>
  ) -> Self {
    ScriptBuilder {
      operator_public_key: operator_public_key.clone(),
      verifier_public_keys: verifier_public_keys.clone()
    }
  }

  pub fn create_timelock_script(
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
  
  pub fn create_peg_in_script(
    &self,
    depositor_public_key: &XOnlyPublicKey,
    evm_address: &EthAddress
  ) -> ScriptBuf {
    let mut builder = Builder::new();
    for verifier_public_key in self.verifier_public_keys.clone() {
      // verify `verifier's` signature
      builder = builder
        .push_x_only_key(&verifier_public_key)
        .push_opcode(OP_CHECKSIGVERIFY);
    }
  
    // verify `operator's` signature
    builder = builder
      .push_x_only_key(&self.operator_public_key)
      .push_opcode(OP_CHECKSIGVERIFY)

      // verify `depositor's` signature
      .push_x_only_key(depositor_public_key)
      .push_opcode(OP_CHECKSIGVERIFY)

      // push envelope with `evm_address` address
      .push_opcode(OP_TRUE)
      .push_opcode(OP_FALSE)
      .push_opcode(OP_IF)
      .push_slice(evm_address)
      .push_opcode(OP_ENDIF);
  
    builder.into_script()
  }
}
