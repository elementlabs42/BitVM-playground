use bitcoin::Amount;

use bitvm::bridge::{
    client::chain::chain::PegOutEvent,
    graphs::base::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::generate_pay_to_pubkey_script_address,
    transactions::{
        base::{BaseTransaction, Input},
        peg_out::PegOutTransaction,
    },
};

use crate::bridge::{faucet::Faucet, helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_peg_out_success() {
    let config = setup_test().await;
    let timestamp = 1722328130u32;

    let input_amount_raw = INITIAL_AMOUNT + FEE_AMOUNT;
    let operator_input_amount = Amount::from_sat(input_amount_raw);

    let operator_funding_utxo_address = generate_pay_to_pubkey_script_address(
        config.operator_context.network,
        &config.operator_context.operator_public_key,
    );
    println!(
        "operator_funding_utxo_address: {:?}",
        operator_funding_utxo_address
    );

    let faucet = Faucet::new();
    faucet
        .fund_input_and_wait(&operator_funding_utxo_address, operator_input_amount)
        .await;

    let operator_funding_outpoint = generate_stub_outpoint(
        &config.client_0,
        &operator_funding_utxo_address,
        operator_input_amount,
    )
    .await;
    println!(
        "operator_funding_utxo.txid: {:?}",
        operator_funding_outpoint.txid
    );
    let stub_event = PegOutEvent {
        source_outpoint: operator_funding_outpoint,
        amount: operator_input_amount,
        timestamp,
        withdrawer_chain_address: config.withdrawer_evm_address,
        withdrawer_public_key_hash: config
            .withdrawer_context
            .withdrawer_public_key
            .pubkey_hash(),
        operator_public_key: config.operator_context.operator_public_key,
        tx_hash: [0u8; 4].into(),
    };
    let input = Input {
        outpoint: operator_funding_outpoint,
        amount: operator_input_amount,
    };

    let peg_out = PegOutTransaction::new(&config.operator_context, &stub_event, input);

    let peg_out_tx = peg_out.finalize();
    let peg_out_txid = peg_out_tx.compute_txid();

    // mine peg-out
    let peg_out_result = config.client_0.esplora.broadcast(&peg_out_tx).await;
    println!("Peg Out Tx result: {:?}", peg_out_result);
    assert!(peg_out_result.is_ok());
    println!("Peg Out Txid: {:?}", peg_out_txid);
}
