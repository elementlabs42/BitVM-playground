use bitcoin::Amount;

use bitvm::bridge::{
    graphs::base::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::generate_pay_to_pubkey_script_address,
    transactions::base::Input,
};

use super::super::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_musig2() {
    let (mut depositor_client, depositor_context, _, _, _, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;
    let (mut operator_client, _, operator_context, _, _, _, _, _, _, _, _, _, _, _, evm_address) =
        setup_test().await;
    let (mut verifier0_client, _, _, verifier0_context, _, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;
    let (mut verifier1_client, _, _, _, verifier1_context, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;

    // Operator: generate graph
    let amount = Amount::from_sat(INITIAL_AMOUNT + FEE_AMOUNT);
    let outpoint = generate_stub_outpoint(
        &operator_client,
        &generate_pay_to_pubkey_script_address(
            operator_context.network,
            &depositor_context.depositor_public_key, // TODO: Add this in operator context.
        ),
        amount,
    )
    .await;

    let graph_id = operator_client
        .create_peg_in_graph(Input { outpoint, amount }, &evm_address)
        .await;
    println!("Operator: Created new graph {}", graph_id);

    println!("Operator: Saving state changes to remote...");
    operator_client.flush().await;

    // Verifier0: push nonces
    println!("Verfier0: Reading state from remote...");
    verifier0_client.sync().await;

    println!("Verfier0: Generating nonces...");
    verifier0_client.push_peg_in_nonces(&graph_id);

    println!("Verfier0: Saving state changes to remote...");
    verifier0_client.sync().await;

    // Verifier1: push nonces
    // Verifier1: presign
    // Verifier0: presign
    // Operator: finalize & verify
}
