use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, PublicKey, XOnlyPublicKey,
};

use super::base::{generate_keys_from_secret, BaseContext};

pub struct VerifierContext {
    pub network: Network,
    pub secp: Secp256k1<All>,

    pub verifier_keypair: Keypair,
    pub verifier_public_key: PublicKey,

    pub n_of_n_public_keys: Vec<PublicKey>,
    pub n_of_n_public_key: PublicKey,
    pub n_of_n_taproot_public_key: XOnlyPublicKey,
}

impl BaseContext for VerifierContext {
    fn network(&self) -> Network { self.network }
    fn secp(&self) -> &Secp256k1<All> { &self.secp }
}

impl VerifierContext {
    pub fn new(
        network: Network,
        verifier_secret: &str,
        n_of_n_public_keys: &Vec<PublicKey>,
        n_of_n_public_key: &PublicKey,
    ) -> Self {
        let (secp, keypair, public_key) = generate_keys_from_secret(network, verifier_secret);

        VerifierContext {
            network,
            secp,

            verifier_keypair: keypair,
            verifier_public_key: public_key,

            n_of_n_public_keys: n_of_n_public_keys.clone(),
            n_of_n_public_key: *n_of_n_public_key,
            n_of_n_taproot_public_key: XOnlyPublicKey::from(*n_of_n_public_key),
        }
    }
}
