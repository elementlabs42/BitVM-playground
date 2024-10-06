use bitcoin::Amount;

use bitvm::bridge::{
    connectors::connector::TaprootConnector,
    constants::SHA256_DIGEST_LENGTH_IN_BYTES,
    graphs::base::ONE_HUNDRED,
    superblock::{get_superblock_message, Superblock, SuperblockHash},
    transactions::{
        base::{BaseTransaction, Input},
        kick_off_2::KickOff2Transaction,
    },
};

use super::super::{helper::generate_stub_outpoint, setup::setup_test};

#[tokio::test]
async fn test_kick_off_2_tx() {
    let (
        client,
        _,
        _,
        operator_context,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        connector_1,
        _,
        _,
        _,
        _,
        _,
        _,
        connector_1_winternitz_secrets,
    ) = setup_test().await;

    let input_value0 = Amount::from_sat(ONE_HUNDRED * 2 / 100);
    let funding_utxo_address0 = connector_1.generate_taproot_address();
    let funding_outpoint0 =
        generate_stub_outpoint(&client, &funding_utxo_address0, input_value0).await;

    let mut kick_off_2_tx = KickOff2Transaction::new(
        &operator_context,
        &connector_1,
        Input {
            outpoint: funding_outpoint0,
            amount: input_value0,
        },
    );

    let sb_hash: SuperblockHash = [0xf0u8; SHA256_DIGEST_LENGTH_IN_BYTES];
    let sb = Superblock {
        height: 123,
        time: 45678,
        weight: 9012345,
    };
    kick_off_2_tx.sign_input_0(
        &operator_context,
        &connector_1,
        &connector_1_winternitz_secrets[&0],
        &get_superblock_message(&sb, &sb_hash),
    );

    let tx = kick_off_2_tx.finalize();
    // println!("Script Path Spend Transaction: {:?}\n", tx);
    let result = client.esplora.broadcast(&tx).await;
    println!("Txid: {:?}", tx.compute_txid());
    println!("Broadcast result: {:?}\n", result);
    // println!("Transaction hex: \n{}", serialize_hex(&tx));
    assert!(result.is_ok());
}
