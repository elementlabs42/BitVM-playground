use musig2::secp::Scalar;

#[tokio::test]
async fn generate_signer_keys() {
    let secret = Scalar::random(&mut rand::rngs::OsRng);
    let secret_string: String = secret
        .serialize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();
    let public_key = secret.base_point_mul();

    println!("Private key:\t{}", secret_string);
    println!("Public key:\t{}", public_key);
}
