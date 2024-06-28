use bitcoin::{Amount, Transaction, Txid};
use bitvm::bridge::{
    client::BitVMClient,
    contexts::operator::OperatorContext,
    graph::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::generate_pay_to_pubkey_script_address,
    transactions::{
        base::{BaseTransaction, Input},
        kick_off::KickOffTransaction,
    },
};

use crate::bridge::helper::generate_stub_outpoint;

pub async fn create_and_mine_kick_off_tx(
    client: &BitVMClient,
    operator_context: &OperatorContext,
) -> (Transaction, Txid) {
    let input_amount_raw = INITIAL_AMOUNT + FEE_AMOUNT;
    let input_amount = Amount::from_sat(input_amount_raw);

    // create kick-off tx
    let kick_off_funding_utxo_address = generate_pay_to_pubkey_script_address(
        operator_context.network,
        &operator_context.operator_public_key,
    );
    let kick_off_funding_outpoint =
        generate_stub_outpoint(&client, &kick_off_funding_utxo_address, input_amount).await;
    let kick_off_input = Input {
        outpoint: kick_off_funding_outpoint,
        amount: input_amount,
    };
    let kick_off = KickOffTransaction::new(&operator_context, kick_off_input);
    let kick_off_tx = kick_off.finalize();
    let kick_off_tx_id = kick_off_tx.compute_txid();

    // mine kick-off tx
    let kick_off_result = client.esplora.broadcast(&kick_off_tx).await;
    assert!(kick_off_result.is_ok());

    return (kick_off_tx, kick_off_tx_id);
}
