use bitcoin::{
    hashes::{ripemd160, ripemd160::Hash as Ripemd160, sha256, sha256::Hash as Sha256, Hash},
    Amount, CompressedPublicKey,
};

use bitvm::bridge::{
    contexts::withdrawer,
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

use crate::bridge::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_peg_out_success() {
    let (client, _, operator_context, _, withdrawer_context, _, _, _, _, _, _, _, _, evm_address) =
        setup_test().await;
    let timestamp = 1722328130u32;

    let pub_key = withdrawer_context.withdrawer_public_key;
    println!("withdrawer pub key: {:?}", hex::encode(pub_key.to_bytes()));
    println!(
        "withdrawer pubkey hash: {:?}",
        hex::encode(pub_key.pubkey_hash())
    );
    let inscription = [
        pub_key.pubkey_hash().as_byte_array().to_vec(),
        timestamp.to_be_bytes().to_vec(),
        evm_address.as_bytes().to_vec(),
    ]
    .concat();
    println!();
    println!(
        "timestamp hex: {:?}",
        hex::encode(timestamp.to_be_bytes().to_vec())
    );
    let script =
        generate_pay_to_pubkey_hash_with_inscription_script(&pub_key, timestamp, &evm_address);
    let script_address = generate_pay_to_pubkey_hash_with_inscription_script_address(
        withdrawer_context.network,
        &pub_key,
        timestamp,
        &evm_address,
    );
    println!("script hex: {:?}", hex::encode(script.as_bytes()));
    println!(
        "script pubkey: {:?}",
        hex::encode(script_address.script_pubkey())
    );
    println!("script address: {:?}", script_address);
    println!();

    let inscription_sha256 = Sha256::hash(&inscription);
    let inscription_hash = Ripemd160::hash(&inscription_sha256.to_byte_array());
    println!("inscription: {:?}", hex::encode(inscription));
    println!(
        "inscription hash: {:?}",
        hex::encode(inscription_hash.to_byte_array().to_vec())
    );

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
