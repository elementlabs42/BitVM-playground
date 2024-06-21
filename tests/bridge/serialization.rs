use crate::bridge::helper::generate_stub_outpoint;

use super::setup::setup_test;
use bitcoin::{Amount, Network};
use bitvm::bridge::{
    components::{
        assert::AssertTransaction,
        bridge::{deserialize, serialize, BridgeTransaction},
        connector_b::ConnectorB,
        helper::Input,
    },
    graph::ONE_HUNDRED,
};
use serde::Serialize;

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

    let json = serialize(&assert_tx);
    assert!(json.len() > 0);
    assert!(json.contains(funding_outpoint.txid.to_string().as_str()));
    assert!(json.contains(funding_outpoint.vout.to_string().as_str()));

    let deserialized_assert_tx = deserialize::<AssertTransaction>(&json);
    assert!(assert_tx == deserialized_assert_tx);
}
