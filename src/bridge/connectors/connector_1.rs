use bitcoin::{
    key::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Address, Network, ScriptBuf, TxIn, XOnlyPublicKey,
};
use bitcoin_script::script;
use serde::{Deserialize, Serialize};

use crate::{
    bridge::constants::SHA256_DIGEST_LENGTH_IN_BYTES,
    signatures::{winternitz::generate_public_key, winternitz_hash::check_hash_sig},
};

use super::{
    super::{
        constants::{NUM_BLOCKS_PER_2_WEEKS, NUM_BLOCKS_PER_6_HOURS, NUM_BLOCKS_PER_DAY},
        scripts::*,
        transactions::base::Input,
        utils::num_blocks_per_network,
    },
    connector::*,
};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Connector1 {
    pub network: Network,
    pub operator_taproot_public_key: XOnlyPublicKey,
    pub n_of_n_taproot_public_key: XOnlyPublicKey,
    pub num_blocks_timelock_0: u32,
    pub num_blocks_timelock_1: u32,
    pub num_blocks_timelock_2: u32,
}

impl Connector1 {
    pub fn new(
        network: Network,
        operator_taproot_public_key: &XOnlyPublicKey,
        n_of_n_taproot_public_key: &XOnlyPublicKey,
    ) -> Self {
        Connector1 {
            network,
            operator_taproot_public_key: operator_taproot_public_key.clone(),
            n_of_n_taproot_public_key: n_of_n_taproot_public_key.clone(),
            num_blocks_timelock_0: num_blocks_per_network(network, NUM_BLOCKS_PER_2_WEEKS),
            num_blocks_timelock_1: num_blocks_per_network(
                network,
                NUM_BLOCKS_PER_2_WEEKS + NUM_BLOCKS_PER_DAY,
            ),
            num_blocks_timelock_2: num_blocks_per_network(network, NUM_BLOCKS_PER_6_HOURS),
        }
    }

    fn generate_taproot_leaf_0_script(&self) -> ScriptBuf {
        let secret = "3076ca1dfc1e383be26d5dd3c0c427340f96139fa8c2520862cf551ec2d670ac"; // TODO: change to pubkey passed by the caller

        script! {
            { self.num_blocks_timelock_0 }
            OP_CSV
            OP_DROP
            { self.operator_taproot_public_key }
            OP_CHECKSIG
            { check_hash_sig(&generate_public_key(secret), SHA256_DIGEST_LENGTH_IN_BYTES) }
        }
        .compile()
    }

    fn generate_taproot_leaf_0_tx_in(&self, input: &Input) -> TxIn {
        generate_timelock_tx_in(input, self.num_blocks_timelock_0)
    }

    fn generate_taproot_leaf_1_script(&self) -> ScriptBuf {
        generate_timelock_taproot_script(
            &self.n_of_n_taproot_public_key,
            self.num_blocks_timelock_1,
        )
    }

    fn generate_taproot_leaf_1_tx_in(&self, input: &Input) -> TxIn {
        generate_timelock_tx_in(input, self.num_blocks_timelock_1)
    }

    fn generate_taproot_leaf_2_script(&self) -> ScriptBuf {
        generate_timelock_taproot_script(
            &self.n_of_n_taproot_public_key,
            self.num_blocks_timelock_2,
        )
    }

    fn generate_taproot_leaf_2_tx_in(&self, input: &Input) -> TxIn {
        generate_timelock_tx_in(input, self.num_blocks_timelock_2)
    }
}

impl TaprootConnector for Connector1 {
    fn generate_taproot_leaf_script(&self, leaf_index: u32) -> ScriptBuf {
        match leaf_index {
            0 => self.generate_taproot_leaf_0_script(),
            1 => self.generate_taproot_leaf_1_script(),
            2 => self.generate_taproot_leaf_2_script(),
            _ => panic!("Invalid leaf index."),
        }
    }

    fn generate_taproot_leaf_tx_in(&self, leaf_index: u32, input: &Input) -> TxIn {
        match leaf_index {
            0 => self.generate_taproot_leaf_0_tx_in(input),
            1 => self.generate_taproot_leaf_1_tx_in(input),
            2 => self.generate_taproot_leaf_2_tx_in(input),
            _ => panic!("Invalid leaf index."),
        }
    }

    fn generate_taproot_spend_info(&self) -> TaprootSpendInfo {
        TaprootBuilder::new()
            .add_leaf(2, self.generate_taproot_leaf_0_script())
            .expect("Unable to add leaf 0")
            .add_leaf(2, self.generate_taproot_leaf_1_script())
            .expect("Unable to add leaf 1")
            .add_leaf(1, self.generate_taproot_leaf_2_script())
            .expect("Unable to add leaf 2")
            .finalize(&Secp256k1::new(), self.n_of_n_taproot_public_key)
            .expect("Unable to finalize taproot")
    }

    fn generate_taproot_address(&self) -> Address {
        Address::p2tr_tweaked(
            self.generate_taproot_spend_info().output_key(),
            self.network,
        )
    }
}
