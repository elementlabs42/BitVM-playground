use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, PublicKey, XOnlyPublicKey,
};

use super::base::{generate_keys_from_secret, BaseContext};

pub struct OperatorContext {
    pub network: Network,
    pub secp: Secp256k1<All>,

    pub keypair: Keypair,
    pub public_key: PublicKey,
    pub taproot_public_key: XOnlyPublicKey,

    pub n_of_n_public_key: PublicKey,
    pub n_of_n_taproot_public_key: XOnlyPublicKey,
}

impl BaseContext for OperatorContext {
    fn network(&self) -> Network { self.network }
    fn secp(&self) -> &Secp256k1<All> { &self.secp }
}

impl OperatorContext {
    pub fn new(network: Network, operator_secret: &str, n_of_n_public_key: &PublicKey) -> Self {
        let (secp, keypair, public_key, taproot_public_key) =
            generate_keys_from_secret(network, operator_secret);

        OperatorContext {
            network,
            secp,

            keypair,
            public_key,
            taproot_public_key,

            n_of_n_public_key: n_of_n_public_key.clone(),
            n_of_n_taproot_public_key: XOnlyPublicKey::from(*n_of_n_public_key),
        }
    }
}
