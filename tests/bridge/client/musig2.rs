use bitcoin::Amount;

use bitvm::bridge::{
    graphs::base::{FEE_AMOUNT, INITIAL_AMOUNT},
    scripts::generate_pay_to_pubkey_script_address,
    transactions::base::Input,
};

use super::super::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_musig2_peg_in() {
    let (mut depositor_client, depositor_context, _, _, _, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;
    let (mut operator_client, _, operator_context, _, _, _, _, _, _, _, _, _, _, _, evm_address) =
        setup_test().await;
    let (mut verifier0_client, _, _, verifier0_context, _, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;
    let (mut verifier1_client, _, _, _, verifier1_context, _, _, _, _, _, _, _, _, _, _) =
        setup_test().await;

    // Depositor: generate graph
    let amount = Amount::from_sat(INITIAL_AMOUNT + FEE_AMOUNT);
    let outpoint = generate_stub_outpoint(
        &depositor_client,
        &generate_pay_to_pubkey_script_address(
            depositor_context.network,
            &depositor_context.public_key,
        ),
        amount,
    )
    .await;

    let graph_id = depositor_client
        .create_peg_in_graph(Input { outpoint, amount }, &evm_address)
        .await;
    println!("Depositor: Created new graph {graph_id}");

    println!("Depositor: Saving state changes to remote...");
    operator_client.flush().await;

    // Verifier0: push nonces
    println!("Verfier0: Reading state from remote...");
    verifier0_client.sync().await;

    println!("Verfier0: Generating nonces...");
    verifier0_client.push_peg_in_nonces(&graph_id);

    println!("Verfier0: Saving state changes to remote...");
    verifier0_client.flush().await;

    // Verifier1: push nonces
    println!("Verfier1: Reading state from remote...");
    verifier1_client.sync().await;

    println!("Verfier1: Generating nonces...");
    verifier1_client.push_peg_in_nonces(&graph_id);

    println!("Verfier1: Saving state changes to remote...");
    verifier1_client.flush().await;

    // Verifier0: presign
    println!("Verfier0: Reading state from remote...");
    verifier0_client.sync().await;

    println!("Verfier0: Pre-signing...");
    verifier0_client.pre_sign_peg_in(&graph_id);

    println!("Verfier0: Saving state changes to remote...");
    verifier0_client.flush().await;

    // Verifier1: presign
    println!("Verfier1: Reading state from remote...");
    verifier1_client.sync().await;

    println!("Verfier1: Pre-signing...");
    verifier1_client.pre_sign_peg_in(&graph_id);

    println!("Verfier1: Saving state changes to remote...");
    verifier1_client.flush().await;

    // Operator: finalize & verify
    println!("Operator: Reading state from remote...");
    operator_client.sync().await;

    // TODO: operator will not be able to finalize because its context lacks the n-of-n public key list.
    println!("Operator: Pre-signing...");
    operator_client.finalize_peg_in(&graph_id);

    println!("Operator: Saving state changes to remote...");
    operator_client.flush().await;
}

async fn test_musig2_peg_out() { todo!() }
