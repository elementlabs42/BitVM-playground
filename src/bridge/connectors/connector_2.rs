use bitcoin::{Address, Network, PublicKey, ScriptBuf, Sequence, TxIn};
use serde::{Deserialize, Serialize};

use super::{
    super::{
        super::bridge::utils::get_num_blocks_per_2_weeks, scripts::*, transactions::base::Input,
    },
    connector::*,
};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Connector2 {
    pub network: Network,
    pub operator_public_key: PublicKey,
    pub num_blocks_timelock: u32,
}

impl Connector2 {
    pub fn new(network: Network, operator_public_key: &PublicKey) -> Self {
        Connector2 {
            network,
            operator_public_key: operator_public_key.clone(),
            num_blocks_timelock: get_num_blocks_per_2_weeks(network),
        }
    }
}

impl P2wshConnector for Connector2 {
    fn generate_script(&self) -> ScriptBuf {
        generate_timelock_script(&self.operator_public_key, self.num_blocks_timelock)
    }

    fn generate_address(&self) -> Address {
        generate_timelock_script_address(
            self.network,
            &self.operator_public_key,
            self.num_blocks_timelock,
        )
    }

    fn generate_tx_in(&self, input: &Input) -> TxIn {
        let mut tx_in = generate_default_tx_in(input);
        tx_in.sequence = Sequence((self.num_blocks_timelock) & 0xFFFFFFFF);
        tx_in
    }
}
