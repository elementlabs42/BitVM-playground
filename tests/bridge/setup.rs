use std::str::FromStr;

use bitcoin::{Network, PublicKey, XOnlyPublicKey};

use bitvm::bridge::{
    client::client::BitVMClient,
    connectors::{
        connector_0::Connector0, connector_1::Connector1, connector_2::Connector2,
        connector_3::Connector3, connector_a::ConnectorA, connector_b::ConnectorB,
        connector_c::ConnectorC, connector_z::ConnectorZ,
    },
    contexts::{
        depositor::DepositorContext, operator::OperatorContext, verifier::VerifierContext,
        withdrawer::WithdrawerContext,
    },
    graphs::base::{
        DEPOSITOR_SECRET, EVM_ADDRESS, N_OF_N_PUBKEY, N_OF_N_PUBKEYS, OPERATOR_PUBKEY,
        OPERATOR_SECRET, VERIFIER0_SECRET, VERIFIER1_SECRET, WITHDRAWER_SECRET,
    },
};

pub async fn setup_test() -> (
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
    // TODO: Add error handling below (when unwrapping).
    let n_of_n_public_key = PublicKey::from_str(N_OF_N_PUBKEY).unwrap();
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    for pkstr in N_OF_N_PUBKEYS {
        n_of_n_public_keys.push(PublicKey::from_str(pkstr).unwrap())
    }
    let operator_public_key = PublicKey::from_str(OPERATOR_PUBKEY).unwrap();

    let depositor_context = DepositorContext::new(network, DEPOSITOR_SECRET, &n_of_n_public_key);
    let operator_context = OperatorContext::new(network, OPERATOR_SECRET, &n_of_n_public_key);
    let verifier0_context = VerifierContext::new(
        network,
        VERIFIER0_SECRET,
        &n_of_n_public_keys,
        &n_of_n_public_key,
        &operator_public_key,
    );
    let verifier1_context = VerifierContext::new(
        network,
        VERIFIER1_SECRET,
        &n_of_n_public_keys,
        &n_of_n_public_key,
        &operator_public_key,
    );
    let withdrawer_context = WithdrawerContext::new(network, WITHDRAWER_SECRET, &n_of_n_public_key);

    let client = BitVMClient::new(
        network,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(&operator_public_key),
        Some(VERIFIER0_SECRET),
        Some(n_of_n_public_keys),
        Some(&n_of_n_public_key),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let connector_a = ConnectorA::new(
        network,
        &operator_context.taproot_public_key,
        &XOnlyPublicKey::from(n_of_n_public_key),
    );
    let connector_b = ConnectorB::new(network, &XOnlyPublicKey::from(n_of_n_public_key));
    let connector_c = ConnectorC::new(network, &XOnlyPublicKey::from(n_of_n_public_key));
    let connector_z = ConnectorZ::new(
        network,
        EVM_ADDRESS,
        &depositor_context.taproot_public_key,
        &XOnlyPublicKey::from(n_of_n_public_key),
    );
    let connector_0 = Connector0::new(network, &PublicKey::from_str(N_OF_N_PUBKEY).unwrap());
    let connector_1 = Connector1::new(network, &PublicKey::from_str(OPERATOR_PUBKEY).unwrap());
    let connector_2 = Connector2::new(network, &PublicKey::from_str(OPERATOR_PUBKEY).unwrap());
    let connector_3 = Connector3::new(network, &PublicKey::from_str(N_OF_N_PUBKEY).unwrap());

    // TODO: Instead of one client with all role contexts in it, return clients limited to only one role for every role.
    // Using those clients in tests will help mimic production environment better.
    return (
        client,
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
