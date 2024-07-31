use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, PublicKey, XOnlyPublicKey,
};

use super::base::{generate_keys_from_secret, BaseContext};

pub struct VerifierContext {
    pub network: Network,
    pub secp: Secp256k1<All>,

    pub keypair: Keypair,
    pub public_key: PublicKey,
    pub verifier_public_keys: Vec<PublicKey>,

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
        verifier_secret: &str,
        verifier_public_keys: &Vec<PublicKey>,
        n_of_n_secret: &str,
        operator_public_key: &PublicKey,
        operator_taproot_public_key: &XOnlyPublicKey,
    ) -> Self {
        let (secp, n_of_n_keypair, n_of_n_public_key, taproot_public_key) =
            generate_keys_from_secret(network, n_of_n_secret);
        let (_, keypair, public_key, _) = generate_keys_from_secret(network, verifier_secret);

        VerifierContext {
            network,
            secp,

            keypair,
            public_key,
            verifier_public_keys: verifier_public_keys.clone(),

            n_of_n_keypair,
            n_of_n_public_key,
            n_of_n_taproot_public_key: taproot_public_key,

            operator_public_key: operator_public_key.clone(),
            operator_taproot_public_key: operator_taproot_public_key.clone(),
        }
    }
}
