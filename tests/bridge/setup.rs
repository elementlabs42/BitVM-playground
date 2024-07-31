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
        DEPOSITOR_SECRET, EVM_ADDRESS, N_OF_N_SECRET, OPERATOR_SECRET, VERIFIER0_SECRET, VERIFIER1_SECRET, WITHDRAWER_SECRET
    },
};

pub async fn setup_test() -> (
    BitVMClient,
    DepositorContext,
    OperatorContext,
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

    let operator_keys = generate_keys_from_secret(network, OPERATOR_SECRET);
    let verifier_keys = generate_keys_from_secret(network, N_OF_N_SECRET);

    let depositor_context = DepositorContext::new(
        network,
        DEPOSITOR_SECRET,
        &verifier_keys.2,
        &verifier_keys.3,
    );
    let operator_context =
        OperatorContext::new(network, OPERATOR_SECRET, &verifier_keys.2, &verifier_keys.3);
    let verifier0_secret = VERIFIER0_SECRET;
    let verifier1_secret = VERIFIER1_SECRET;
    let mut verifier_public_keys: Vec<PublicKey> = Vec::new();
    verifier_public_keys.push(generate_keys_from_secret(network, verifier0_secret).2);
    verifier_public_keys.push(generate_keys_from_secret(network, verifier1_secret).2);
    let verifier0_context =
        VerifierContext::new(network, verifier0_secret, &verifier_public_keys, N_OF_N_SECRET, &operator_keys.2, &operator_keys.3);
    let verifier1_context =
        VerifierContext::new(network, verifier1_secret, &verifier_public_keys, N_OF_N_SECRET, &operator_keys.2, &operator_keys.3);
    let withdrawer_context = WithdrawerContext::new(
        network,
        WITHDRAWER_SECRET,
        &verifier_keys.2,
        &verifier_keys.3,
    );

    let client = BitVMClient::new(
        network,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(verifier0_secret),
        Some(verifier_public_keys),
        Some(N_OF_N_SECRET),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let connector_a = ConnectorA::new(
        network,
        &operator_context.operator_taproot_public_key,
        &verifier0_context.n_of_n_taproot_public_key,
    );
    let connector_b = ConnectorB::new(network, &verifier0_context.n_of_n_taproot_public_key);
    let connector_c = ConnectorC::new(network, &verifier0_context.n_of_n_taproot_public_key);
    let connector_z = ConnectorZ::new(
        network,
        EVM_ADDRESS,
        &depositor_context.depositor_taproot_public_key,
        &verifier0_context.n_of_n_taproot_public_key,
    );
    let connector_0 = Connector0::new(network, &verifier0_context.n_of_n_public_key);
    let connector_1 = Connector1::new(network, &operator_context.operator_public_key);
    let connector_2 = Connector2::new(network, &operator_context.operator_public_key);
    let connector_3 = Connector3::new(network, &verifier0_context.n_of_n_public_key);

    return (
        client,
        depositor_context,
        operator_context,
        verifier0_context,
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
