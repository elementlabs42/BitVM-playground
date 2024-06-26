use bitcoin::{consensus::encode::serialize_hex, Amount, OutPoint};

use bitvm::bridge::{
    connectors::connector::TaprootConnector,
    graph::{FEE_AMOUNT, INITIAL_AMOUNT},
    transactions::{
        base::{BaseTransaction, Input},
        peg_in_refund::PegInRefundTransaction,
    },
};

use super::super::setup::setup_test;

#[tokio::test]
async fn test_peg_in_refund_tx() {
    let (client, depositor_context, _, _, _, _, _, _, connector_z, _, _, _, _) = setup_test();

    let evm_address = String::from("evm address");
    let input_amount_raw = INITIAL_AMOUNT + FEE_AMOUNT;
    let input_amount = Amount::from_sat(input_amount_raw);
    let funding_address = connector_z.generate_taproot_address();

    let funding_utxo_0 = client
        .get_initial_utxo(funding_address.clone(), input_amount)
        .await
        .unwrap_or_else(|| {
            panic!(
                "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                funding_address.clone(),
                input_amount_raw
            );
        });
    let funding_outpoint_0 = OutPoint {
        txid: funding_utxo_0.txid,
        vout: funding_utxo_0.vout,
    };

    let input_amount = Amount::from_sat(INITIAL_AMOUNT + FEE_AMOUNT);

    let input = Input {
        outpoint: funding_outpoint_0,
        amount: input_amount,
    };

    let peg_in_refund_tx = PegInRefundTransaction::new(&depositor_context, input, evm_address);

    let tx = peg_in_refund_tx.finalize();
    println!("Script Path Spend Transaction: {:?}\n", tx);
    let result = client.esplora.broadcast(&tx).await;
    println!("Txid: {:?}", tx.compute_txid());
    println!("Broadcast result: {:?}\n", result);
    println!("Transaction hex: \n{}", serialize_hex(&tx));
    assert!(result.is_ok());
}
