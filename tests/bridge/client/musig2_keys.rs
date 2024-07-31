use musig2::{
    secp::{Point, Scalar},
    KeyAggContext,
};

#[tokio::test]
async fn generate_signer_keys() {
    let mut public_keys: Vec<Point> = Vec::new();
    for i in 0..2 {
        let secret = Scalar::random(&mut rand::rngs::OsRng);
        let secret_string: String = secret
            .serialize()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();
        let public_key = secret.base_point_mul();
        public_keys.push(public_key);

        println!("Signer{i} private key:\t{secret_string}");
        println!("Signer{i} public key:\t{public_key}");
    }

    let key_agg_ctx = KeyAggContext::new(public_keys).unwrap();
    let aggregated_key: Point = key_agg_ctx.aggregated_pubkey();

    println!("N-of-n public Key:\t{aggregated_key}");
}
