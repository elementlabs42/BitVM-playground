use bitcoin::{Network, PublicKey};

use bitvm::bridge::{
    client::client::BitVMClient,
    connectors::{
        connector_0::Connector0, connector_1::Connector1, connector_2::Connector2,
        connector_3::Connector3, connector_a::ConnectorA, connector_b::ConnectorB,
        connector_c::ConnectorC, connector_z::ConnectorZ,
    },
    contexts::{
        base::generate_keys_from_secret, depositor::DepositorContext, operator::OperatorContext,
        verifier::VerifierContext, withdrawer::WithdrawerContext,
    },
    graphs::base::{
        DEPOSITOR_SECRET, EVM_ADDRESS, OPERATOR_SECRET, VERIFIER0_SECRET, VERIFIER1_SECRET,
        WITHDRAWER_SECRET,
    },
};

pub async fn setup_test() -> (
    BitVMClient,
    BitVMClient,
    DepositorContext,
    OperatorContext,
    VerifierContext,
    VerifierContext,
    WithdrawerContext,
    ConnectorA,
    ConnectorB,
    ConnectorC,
    ConnectorZ,
    Connector0,
    Connector1,
    Connector2,
    Connector3,
    String,
) {
    let network = Network::Testnet;

    let (_, _, verifier0_public_key) = generate_keys_from_secret(network, VERIFIER0_SECRET);
    let (_, _, verifier1_public_key) = generate_keys_from_secret(network, VERIFIER1_SECRET);
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    n_of_n_public_keys.push(verifier0_public_key);
    n_of_n_public_keys.push(verifier1_public_key);

    let depositor_context = DepositorContext::new(network, DEPOSITOR_SECRET, &n_of_n_public_keys);
    let operator_context = OperatorContext::new(network, OPERATOR_SECRET, &n_of_n_public_keys);

    let verifier0_context = VerifierContext::new(network, VERIFIER0_SECRET, &n_of_n_public_keys);
    let verifier1_context = VerifierContext::new(network, VERIFIER1_SECRET, &n_of_n_public_keys);
    let withdrawer_context =
        WithdrawerContext::new(network, WITHDRAWER_SECRET, &n_of_n_public_keys);

    let client0 = BitVMClient::new(
        network,
        &n_of_n_public_keys,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(VERIFIER0_SECRET),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let client1 = BitVMClient::new(
        network,
        &n_of_n_public_keys,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(VERIFIER1_SECRET),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let connector_a = ConnectorA::new(
        network,
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_b = ConnectorB::new(network, &operator_context.n_of_n_taproot_public_key);
    let connector_c = ConnectorC::new(network, &operator_context.n_of_n_taproot_public_key);
    let connector_z = ConnectorZ::new(
        network,
        EVM_ADDRESS,
        &depositor_context.depositor_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_0 = Connector0::new(network, &operator_context.n_of_n_public_key);
    let connector_1 = Connector1::new(network, &operator_context.operator_public_key);
    let connector_2 = Connector2::new(network, &operator_context.operator_public_key);
    let connector_3 = Connector3::new(network, &operator_context.n_of_n_public_key);

    return (
        client0,
        client1,
        depositor_context,
        operator_context,
        verifier0_context,
        verifier1_context,
        withdrawer_context,
        connector_a,
        connector_b,
        connector_c,
        connector_z,
        connector_0,
        connector_1,
        connector_2,
        connector_3,
        EVM_ADDRESS.to_string(),
    );
}
