use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, PrivateKey, PublicKey,
};

pub trait BaseContext {
    fn network(&self) -> Network;
    fn secp(&self) -> &Secp256k1<All>;
}

pub fn generate_keys_from_secret(
    network: Network,
    secret: &str,
) -> (Secp256k1<All>, Keypair, PublicKey) {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_seckey_str(&secp, secret).unwrap();
    let private_key = PrivateKey::new(keypair.secret_key(), network);
    let public_key = PublicKey::from_private_key(&secp, &private_key);

    (secp, keypair, public_key)
}
