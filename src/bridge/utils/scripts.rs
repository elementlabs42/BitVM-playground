use crate::treepp::*;
use bitcoin::XOnlyPublicKey;

pub fn generate_pre_sign_script(pubkey: XOnlyPublicKey) -> Script {
  script! {
    { pubkey }
    OP_CHECKSIG
  }
}
