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
        base::generate_keys_from_secret, depositor::DepositorContext, operator::OperatorContext,
        verifier::VerifierContext, withdrawer::WithdrawerContext,
    },
    graphs::base::{
        DEPOSITOR_SECRET, EVM_ADDRESS, N_OF_N_PUBKEY, N_OF_N_PUBKEYS, N_OF_N_SECRET,
        OPERATOR_PUBKEY, OPERATOR_SECRET, VERIFIER0_SECRET, VERIFIER1_SECRET, WITHDRAWER_SECRET,
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

    let depositor_context = DepositorContext::new(network, DEPOSITOR_SECRET, N_OF_N_PUBKEY);
    let operator_context = OperatorContext::new(network, OPERATOR_SECRET, N_OF_N_PUBKEY);
    let verifier0_context = VerifierContext::new(
        network,
        VERIFIER0_SECRET,
        N_OF_N_PUBKEYS,
        N_OF_N_PUBKEY,
        OPERATOR_PUBKEY,
    );
    let verifier1_context = VerifierContext::new(
        network,
        VERIFIER1_SECRET,
        N_OF_N_PUBKEYS,
        N_OF_N_PUBKEY,
        OPERATOR_PUBKEY,
    );
    let withdrawer_context = WithdrawerContext::new(network, WITHDRAWER_SECRET, N_OF_N_PUBKEY);

    let client = BitVMClient::new(
        network,
        Some(DEPOSITOR_SECRET),
        Some(OPERATOR_SECRET),
        Some(OPERATOR_PUBKEY),
        Some(VERIFIER0_SECRET),
        Some(N_OF_N_PUBKEYS),
        Some(N_OF_N_PUBKEY),
        Some(WITHDRAWER_SECRET),
    )
    .await;

    let connector_a = ConnectorA::new(
        network,
        &operator_context.taproot_public_key,
        &XOnlyPublicKey::from(N_OF_N_PUBKEY),
    );
    let connector_b = ConnectorB::new(network, &XOnlyPublicKey::from(N_OF_N_PUBKEY));
    let connector_c = ConnectorC::new(network, &XOnlyPublicKey::from(N_OF_N_PUBKEY));
    let connector_z = ConnectorZ::new(
        network,
        EVM_ADDRESS,
        &depositor_context.taproot_public_key,
        &XOnlyPublicKey::from(N_OF_N_PUBKEY),
    );
    let connector_0 = Connector0::new(network, N_OF_N_PUBKEY);
    let connector_1 = Connector1::new(network, OPERATOR_PUBKEY);
    let connector_2 = Connector2::new(network, OPERATOR_PUBKEY);
    let connector_3 = Connector3::new(network, N_OF_N_PUBKEY);

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
