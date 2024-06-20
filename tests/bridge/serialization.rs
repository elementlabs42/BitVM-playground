use crate::bridge::helper::generate_stub_outpoint;

use super::setup::setup_test;
use bitcoin::{Amount, Network};
use bitvm::bridge::{
    components::{
        assert::AssertTransaction, bridge::BridgeTransaction, connector_b::ConnectorB,
        helper::Input,
    },
    graph::ONE_HUNDRED,
};

#[tokio::test]
async fn test_txn_serialization() {
    let (client, context) = setup_test();

    let connector_b = ConnectorB::new(
        Network::Testnet,
        &context.n_of_n_taproot_public_key.unwrap(),
    );

    let input_value = Amount::from_sat(ONE_HUNDRED * 2 / 100);
    let funding_outpoint = generate_stub_outpoint(
        &client,
        &connector_b.generate_taproot_address(),
        input_value,
    )
    .await;

    let mut assert_tx = AssertTransaction::new(
        &context,
        Input {
            outpoint: funding_outpoint,
            amount: input_value,
        },
    );

    assert_tx.pre_sign(&context);

    let json = serde_json::to_string(&assert_tx).unwrap();
    assert!(json.len() > 0);
    assert!(json.contains(funding_outpoint.txid.to_string().as_str()));
    assert!(json.contains(funding_outpoint.vout.to_string().as_str()));

    let deserialized_assert_tx: AssertTransaction = serde_json::from_str(&json).unwrap();
    assert!(assert_tx == deserialized_assert_tx);
}
