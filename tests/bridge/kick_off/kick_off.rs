use bitcoin::{
    consensus::encode::serialize_hex,
    Amount, OutPoint,
};

use bitvm::bridge::{
    graph::FEE_AMOUNT,
    scripts::generate_pay_to_pubkey_script_address,
    transactions::{
        bridge::{BridgeTransaction, Input},
        kick_off::KickOffTransaction,
    },
};

use super::super::setup::setup_test;

#[tokio::test]
async fn test_kick_off_tx() {
    let (client, context, _, _, _, _, _, _) = setup_test();

    let kickoff_input_amount = Amount::from_int_btc(2);
    let fee_amount = Amount::from_sat(FEE_AMOUNT); // TODO: verify this is the correct fee amt
    let total_input_amount = kickoff_input_amount + fee_amount;
    let funding_address = generate_pay_to_pubkey_script_address(
        context.network,
        &context.operator_public_key.unwrap(),
    );

    let funding_utxo = client
        .get_initial_utxo(funding_address.clone(), total_input_amount)
        .await
        .unwrap_or_else(|| {
            panic!(
                "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                funding_address.clone(),
                total_input_amount.to_sat()
            );
        });
    let funding_outpoint = OutPoint {
        txid: funding_utxo.txid,
        vout: funding_utxo.vout,
    };
    let input = Input {
      outpoint: funding_outpoint,
      amount: kickoff_input_amount,
    };

    let mut kickoff_tx = KickOffTransaction::new(&context, input);

    let tx = kickoff_tx.finalize(&context);
    println!("Kick-Off Transaction: {:?}\n", tx);
    let result = client.esplora.broadcast(&tx).await;
    println!("Txid: {:?}", tx.compute_txid());
    println!("Broadcast result: {:?}\n", result);
    println!("Transaction hex: \n{}", serialize_hex(&tx));
    assert!(result.is_ok());
}
