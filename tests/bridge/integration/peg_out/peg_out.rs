use std::time::Duration;

use bitcoin::{Address, Amount, CompressedPublicKey, OutPoint};

use bitvm::bridge::{
    connectors::{connector::P2wshConnector, connector_0::Connector0},
    contexts::operator,
    graphs::base::{DUST_AMOUNT, FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::{
        generate_p2wpkh_address, generate_pay_to_pubkey_script,
        generate_pay_to_pubkey_script_address,
    },
    transactions::{
        base::{BaseTransaction, Input},
        peg_in_confirm::PegInConfirmTransaction,
        peg_in_deposit::PegInDepositTransaction,
        peg_in_refund::PegInRefundTransaction,
        peg_out::PegOutTransaction,
    },
};
use esplora_client::Error;
use serde::Serialize;
use tokio::time::sleep;

use crate::bridge::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_peg_out_success() {
    let (client, _, operator_context, _, withdrawer_context, _, _, _, _, _, _, _, _, evm_address) =
        setup_test().await;

    // let connector_z = ConnectorZ::new(
    //     depositor_context.network,
    //     &evm_address,
    //     &depositor_context.depositor_taproot_public_key,
    //     &depositor_context.n_of_n_taproot_public_key,
    // );

    // println!("n_of_n pub key: {:?}", hex::encode(depositor_context.n_of_n_taproot_public_key.serialize()));
    // println!("depositor pub key: {:?}", hex::encode(depositor_context.depositor_taproot_public_key.serialize()));
    // println!("withdrawer pub key: {:?}", hex::encode(withdrawer_context.withdrawer_public_key.to_bytes()));
    // println!("withdrawer x pub key: {:?}", hex::encode(withdrawer_context.withdrawer_taproot_public_key.serialize()));
    // println!();
    // println!("operator pub key: {:?}", hex::encode(operator_context.operator_public_key.to_bytes()));
    // println!("operator x pub key: {:?}", hex::encode(operator_context.operator_taproot_public_key.serialize()));
    // println!();
    // println!("withdrawer pay script pub key: {:?}", hex::encode(generate_pay_to_pubkey_script(&withdrawer_context.withdrawer_public_key).as_bytes()));
    // println!("withdrawer address: {:?}", generate_p2wpkh_address(withdrawer_context.network, &withdrawer_context.withdrawer_public_key));
    // println!("withdrawer address: {:?}", &generate_p2wpkh_address(withdrawer_context.network, &withdrawer_context.withdrawer_public_key).script_pubkey());
    // println!("withdrawer address: {:?}", Address::p2pkh(&CompressedPublicKey::try_from(withdrawer_context.withdrawer_public_key).expect("Could not compress public key"), withdrawer_context.network).script_pubkey());
    // println!("withdrawer pubkey hash: {:?}", hex::encode(CompressedPublicKey::try_from(withdrawer_context.withdrawer_public_key).expect("Could not compress public key").to_bytes()));
    // println!();
    // println!("vout pubkey: {:?}", generate_pay_to_pubkey_hash_script_address(withdrawer_context.network, &generate_p2wpkh_address(withdrawer_context.network, &withdrawer_context.withdrawer_public_key)));
    // println!("time lock script: {:?}", hex::encode(connector_z.generate_taproot_leaf0_script().as_bytes()));
    // println!("evm address script: {:?}", hex::encode(connector_z.generate_taproot_leaf1_script().as_bytes()));
    // println!("z taproot address: {:?}", connector_z.generate_taproot_address());
    // println!("z taproot pubkey: {:?}", hex::encode(connector_z.generate_taproot_spend_info().output_key().serialize()));
    // println!("z taproot pubkey: {:?}", connector_z.generate_taproot_spend_info().output_key());

    // operator funding 0
    // let withdrawer_input_amount = Amount::from_sat(DUST_AMOUNT);
    // let withdrawer_funding_utxo_address = generate_pay_to_pubkey_script_address(
    //         withdrawer_context.network,
    //         &withdrawer_context.withdrawer_public_key,
    // );
    // println!("withdrawer_funding_utxo_address: {:?}", withdrawer_funding_utxo_address);
    // let withdrawer_funding_outpoint = generate_stub_outpoint(
    //     &client,
    //     &withdrawer_funding_utxo_address,
    //     withdrawer_input_amount,
    // )
    // .await;
    // println!("withdrawer_funding_utxo.txid: {:?}", withdrawer_funding_outpoint.txid);
    // let withdrawer_input = Input {
    //     outpoint: withdrawer_funding_outpoint,
    //     amount: withdrawer_input_amount,
    // };

    let input_amount_raw = INITIAL_AMOUNT + FEE_AMOUNT;
    let operator_input_amount = Amount::from_sat(input_amount_raw);

    // operator funding 99
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
        // withdrawer_input,
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
