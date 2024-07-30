use bitcoin::{Amount, CompressedPublicKey};

use bitvm::bridge::{
    graphs::base::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::{
        generate_pay_to_pubkey_hash_with_inscription_script,
        generate_pay_to_pubkey_hash_with_inscription_script_address,
        generate_pay_to_pubkey_script_address,
    },
    transactions::{
        base::{BaseTransaction, Input},
        peg_out::PegOutTransaction,
    },
};
use sha2::{Digest, Sha256};

use crate::bridge::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_peg_out_success() {
    let (client, _, operator_context, _, withdrawer_context, _, _, _, _, _, _, _, _, evm_address) =
        setup_test().await;
    let timestamp = 1722328130u32;

    println!(
        "withdrawer pub key: {:?}",
        hex::encode(withdrawer_context.withdrawer_public_key.to_bytes())
    );
    println!(
        "withdrawer x pub key: {:?}",
        hex::encode(withdrawer_context.withdrawer_taproot_public_key.serialize())
    );
    println!(
        "withdrawer pubkey: {:?}",
        hex::encode(
            CompressedPublicKey::try_from(withdrawer_context.withdrawer_public_key)
                .expect("Could not compress public key")
                .to_bytes()
        )
    );
    println!(
        "withdrawer pubkey hash: {:?}",
        hex::encode(withdrawer_context.withdrawer_public_key.pubkey_hash())
    );
    let inscription = [
        withdrawer_context.withdrawer_public_key.to_bytes(),
        timestamp.to_be_bytes().to_vec(),
        evm_address.as_bytes().to_vec(),
    ]
    .concat();
    let mut inscription_hasher = Sha256::new();
    inscription_hasher.update(&inscription);
    let inscription_hash = inscription_hasher.finalize();
    println!("inscription: {:?}", hex::encode(inscription));
    println!(
        "inscription hash: {:?}",
        hex::encode(inscription_hash.to_vec())
    );
    println!();
    let script = generate_pay_to_pubkey_hash_with_inscription_script(
        &withdrawer_context.withdrawer_public_key,
        timestamp,
        &evm_address,
    );
    let script_address = generate_pay_to_pubkey_hash_with_inscription_script_address(
        withdrawer_context.network,
        &withdrawer_context.withdrawer_public_key,
        timestamp,
        &evm_address,
    );
    println!("script hex: {:?}", hex::encode(script.as_bytes()));
    let mut script_hasher = Sha256::new();
    script_hasher.update(&script.as_bytes());
    println!(
        "script sha256(hex): {:?}",
        hex::encode(script_hasher.finalize().to_vec())
    );
    println!(
        "script pubkey: {:?}",
        hex::encode(script_address.script_pubkey())
    );
    println!("script address: {:?}", script_address);
    println!(">>>>>>>>>>>>>>>>");
    println!();

    let input_amount_raw = INITIAL_AMOUNT + FEE_AMOUNT;
    let operator_input_amount = Amount::from_sat(input_amount_raw);

    let operator_funding_utxo_address = generate_pay_to_pubkey_script_address(
        operator_context.network,
        &operator_context.operator_public_key,
    );
    println!(
        "operator_funding_utxo_address: {:?}",
        operator_funding_utxo_address
    );
    let operator_funding_outpoint = generate_stub_outpoint(
        &client,
        &operator_funding_utxo_address,
        operator_input_amount,
    )
    .await;
    println!(
        "operator_funding_utxo.txid: {:?}",
        operator_funding_outpoint.txid
    );
    let operator_input = Input {
        outpoint: operator_funding_outpoint,
        amount: operator_input_amount,
    };

    let peg_out = PegOutTransaction::new(
        &operator_context,
        &withdrawer_context.withdrawer_public_key,
        &evm_address,
        timestamp,
        operator_input,
    );

    let peg_out_tx = peg_out.finalize();
    let peg_out_tx_id = peg_out_tx.compute_txid();

    // mine peg-out
    let peg_out_result = client.esplora.broadcast(&peg_out_tx).await;
    println!("Peg Out Tx result: {:?}", peg_out_result);
    assert!(peg_out_result.is_ok());
    println!("Peg Out Txid: {:?}", peg_out_tx_id);
}
