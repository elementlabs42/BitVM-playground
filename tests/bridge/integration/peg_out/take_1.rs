use std::time::Duration;

use bitcoin::{Address, Amount, OutPoint};
use bitvm::bridge::{
    connectors::connector::TaprootConnector,
    graphs::base::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::generate_pay_to_pubkey_script_address,
    transactions::{
        base::{BaseTransaction, Input},
        take_1::Take1Transaction,
    },
};
use tokio::time::sleep;

use crate::bridge::{helper::verify_funding_inputs, setup::setup_test};

use super::utils::{create_and_mine_kick_off_tx, create_and_mine_peg_in_confirm_tx};

#[tokio::test]
async fn test_take_1_success() {
    let (
        client,
        _,
        depositor_context,
        operator_context,
        verifier_0_context,
        verifier_1_context,
        _,
        _,
        _,
        _,
        connector_z,
        _,
        _,
        _,
        _,
        depositor_evm_address,
        _,
    ) = setup_test().await;

    // verify funding inputs
    let mut funding_inputs: Vec<(&Address, Amount)> = vec![];

    let deposit_input_amount = Amount::from_sat(INITIAL_AMOUNT + FEE_AMOUNT);
    let peg_in_confirm_funding_address = connector_z.generate_taproot_address();
    funding_inputs.push((&peg_in_confirm_funding_address, deposit_input_amount));

    let kick_off_input_amount = Amount::from_sat(INITIAL_AMOUNT + FEE_AMOUNT);
    let kick_off_funding_utxo_address = generate_pay_to_pubkey_script_address(
        operator_context.network,
        &operator_context.operator_public_key,
    );
    funding_inputs.push((&kick_off_funding_utxo_address, kick_off_input_amount));

    verify_funding_inputs(&client, &funding_inputs).await;

    // peg-in confirm
    let (peg_in_confirm_tx, peg_in_confirm_tx_id) = create_and_mine_peg_in_confirm_tx(
        &client,
        &depositor_context,
        &verifier_0_context,
        &verifier_1_context,
        &depositor_evm_address,
        &peg_in_confirm_funding_address,
        deposit_input_amount,
    )
    .await;

    // kick-off
    let (kick_off_tx, kick_off_tx_id) = create_and_mine_kick_off_tx(
        &client,
        &operator_context,
        &kick_off_funding_utxo_address,
        kick_off_input_amount,
    )
    .await;

    // take 1
    let connector_0_input = Input {
        outpoint: OutPoint {
            txid: peg_in_confirm_tx_id,
            vout: 0,
        },
        amount: peg_in_confirm_tx.output[0].value,
    };
    let connector_1_input = Input {
        outpoint: OutPoint {
            txid: kick_off_tx_id,
            vout: 0,
        },
        amount: kick_off_tx.output[0].value,
    };
    let connector_a_input = Input {
        outpoint: OutPoint {
            txid: kick_off_tx_id,
            vout: 1,
        },
        amount: kick_off_tx.output[1].value,
    };
    let connector_b_input = Input {
        outpoint: OutPoint {
            txid: kick_off_tx_id,
            vout: 2,
        },
        amount: kick_off_tx.output[2].value,
    };

    let mut take_1 = Take1Transaction::new(
        &operator_context,
        connector_0_input,
        connector_1_input,
        connector_a_input,
        connector_b_input,
    );

    let secret_nonces_0 = take_1.push_nonces(&verifier_0_context);
    let secret_nonces_1 = take_1.push_nonces(&verifier_1_context);

    take_1.pre_sign(&verifier_0_context, &secret_nonces_0);
    take_1.pre_sign(&verifier_1_context, &secret_nonces_1);

    let take_1_tx = take_1.finalize();
    let take_1_tx_id = take_1_tx.compute_txid();

    // mine take 1
    sleep(Duration::from_secs(60)).await;
    let take_1_result = client.esplora.broadcast(&take_1_tx).await;
    assert!(take_1_result.is_ok());

    // operator balance
    let operator_address = generate_pay_to_pubkey_script_address(
        operator_context.network,
        &operator_context.operator_public_key,
    );
    let operator_utxos = client
        .esplora
        .get_address_utxo(operator_address.clone())
        .await
        .unwrap();
    let operator_utxo = operator_utxos
        .clone()
        .into_iter()
        .find(|x| x.txid == take_1_tx_id);

    // assert
    assert!(operator_utxo.is_some());
}