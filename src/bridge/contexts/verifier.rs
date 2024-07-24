use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, PublicKey, XOnlyPublicKey,
};
use musig2::secp::{Point, Scalar};

use super::base::{generate_keys_from_secret, BaseContext};

pub struct VerifierContext {
    pub network: Network,
    pub secp: Secp256k1<All>,

    priv_key: Scalar,
    pub public_key: Point,

    pub n_of_n_keypair: Keypair,
    pub n_of_n_public_key: PublicKey,
    pub n_of_n_taproot_public_key: XOnlyPublicKey,

    pub operator_public_key: PublicKey,
    pub operator_taproot_public_key: XOnlyPublicKey,
}

impl BaseContext for VerifierContext {
    fn network(&self) -> Network { self.network }
    fn secp(&self) -> &Secp256k1<All> { &self.secp }
}

impl VerifierContext {
    pub fn new(
        network: Network,
        private_key: Scalar,
        n_of_n_secret: &str,
        operator_public_key: &PublicKey,
        operator_taproot_public_key: &XOnlyPublicKey,
    ) -> Self {
        let (secp, keypair, public_key, taproot_public_key) =
            generate_keys_from_secret(network, n_of_n_secret);

        VerifierContext {
            network,
            secp,

            priv_key: private_key,
            public_key: private_key.base_point_mul(),

            n_of_n_keypair: keypair,
            n_of_n_public_key: public_key,
            n_of_n_taproot_public_key: taproot_public_key,

            operator_public_key: operator_public_key.clone(),
            operator_taproot_public_key: operator_taproot_public_key.clone(),
        }
    }
}
