use bitcoin::{Network, PublicKey};

use bitvm::bridge::{
    client::client::BitVMClient,
    connectors::{
        connector_0::Connector0, connector_1::Connector1, connector_2::Connector2,
        connector_3::Connector3, connector_4::Connector4, connector_5::Connector5,
        connector_a::ConnectorA, connector_b::ConnectorB, connector_c::ConnectorC,
        connector_z::ConnectorZ,
    },
    contexts::{
        base::generate_keys_from_secret, depositor::DepositorContext, operator::OperatorContext,
        verifier::VerifierContext, withdrawer::WithdrawerContext,
    },
    graphs::base::{
        DEPOSITOR_EVM_ADDRESS, DEPOSITOR_SECRET, OPERATOR_SECRET, VERIFIER_0_SECRET,
        VERIFIER_1_SECRET, WITHDRAWER_EVM_ADDRESS, WITHDRAWER_SECRET,
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
    Connector4,
    Connector5,
    String,
    String,
) {
    let network = Network::Testnet;

    let (_, _, verifier_0_public_key) = generate_keys_from_secret(network, VERIFIER_0_SECRET);
    let (_, _, verifier_1_public_key) = generate_keys_from_secret(network, VERIFIER_1_SECRET);
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    n_of_n_public_keys.push(verifier_0_public_key);
    n_of_n_public_keys.push(verifier_1_public_key);

    let depositor_context = DepositorContext::new(network, DEPOSITOR_SECRET, &n_of_n_public_keys);
    let operator_context = OperatorContext::new(network, OPERATOR_SECRET, &n_of_n_public_keys);

    let verifier_0_context = VerifierContext::new(network, VERIFIER_0_SECRET, &n_of_n_public_keys);
    let verifier_1_context = VerifierContext::new(network, VERIFIER_1_SECRET, &n_of_n_public_keys);
    let withdrawer_context =
        WithdrawerContext::new(network, WITHDRAWER_SECRET, &n_of_n_public_keys);

    let client_0 = BitVMClient::new(
        network,
        &n_of_n_public_keys,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(VERIFIER_0_SECRET),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let client_1 = BitVMClient::new(
        network,
        &n_of_n_public_keys,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(VERIFIER_1_SECRET),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let connector_a = ConnectorA::new(
        network,
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_b = ConnectorB::new(network, &operator_context.n_of_n_taproot_public_key);
    let connector_c = ConnectorC::new(network, &operator_context.operator_taproot_public_key);
    let connector_z = ConnectorZ::new(
        network,
        DEPOSITOR_EVM_ADDRESS,
        &depositor_context.depositor_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_0 = Connector0::new(network, &operator_context.n_of_n_taproot_public_key);
    let connector_1 = Connector1::new(
        network,
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_2 = Connector2::new(
        network,
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_3 = Connector3::new(network, &operator_context.operator_public_key);
    let connector_4 = Connector4::new(network, &operator_context.operator_public_key);
    let connector_5 = Connector5::new(network, &operator_context.n_of_n_taproot_public_key);

    return (
        client_0,
        client_1,
        depositor_context,
        operator_context,
        verifier_0_context,
        verifier_1_context,
        withdrawer_context,
        connector_a,
        connector_b,
        connector_c,
        connector_z,
        connector_0,
        connector_1,
        connector_2,
        connector_3,
        connector_4,
        connector_5,
        DEPOSITOR_EVM_ADDRESS.to_string(),
        WITHDRAWER_EVM_ADDRESS.to_string(),
    );
}
