use crate::treepp::Script;

pub type LockScript = fn(u32) -> Script;
pub type UnlockWitness = fn(u32) -> Vec<Vec<u8>>;