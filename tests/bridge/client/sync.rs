use bitcoin::{consensus::encode::serialize_hex, Amount, OutPoint};

use bitvm::bridge::{
    connectors::connector::TaprootConnector,
    graphs::base::{DUST_AMOUNT, INITIAL_AMOUNT},
    scripts::{generate_pay_to_pubkey_script, generate_pay_to_pubkey_script_address},
    transactions::{
        base::{BaseTransaction, Input, InputWithScript},
        challenge::ChallengeTransaction,
    },
};

use super::super::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_sync() {
    let (
        mut client,
        depositor_context,
        operator_context,
        _,
        _,
        connector_a,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
    ) = setup_test().await;

    client.sync();

    // let tx = challenge_tx.finalize();
    // println!("Script Path Spend Transaction: {:?}\n", tx);
    // let result = client.esplora.broadcast(&tx).await;
    // println!("Txid: {:?}", tx.compute_txid());
    // println!("Broadcast result: {:?}\n", result);
    // println!("Transaction hex: \n{}", serialize_hex(&tx));
    // assert!(result.is_ok());

    // // assert refund balance
    // let challenge_tx_id = tx.compute_txid();
    // let refund_utxos = client
    //     .esplora
    //     .get_address_utxo(refund_address)
    //     .await
    //     .unwrap();
    // let refund_utxo = refund_utxos
    //     .clone()
    //     .into_iter()
    //     .find(|x| x.txid == challenge_tx_id);
    // assert!(refund_utxo.is_some());
    // assert_eq!(
    //     refund_utxo.unwrap().value,
    //     amount_1 * 2 - input_amount_crowdfunding_total
    // );
}
