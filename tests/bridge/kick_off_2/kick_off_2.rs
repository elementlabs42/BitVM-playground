use std::str::FromStr;

use bitcoin::{
    block::{Header, Version},
    Amount, BlockHash, CompactTarget, TxMerkleNode,
};

use bitvm::bridge::{
    connectors::base::TaprootConnector,
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
    let config = setup_test().await;

    let input_value0 = Amount::from_sat(ONE_HUNDRED * 2 / 100);
    let funding_utxo_address0 = config.connector_1.generate_taproot_address();
    let funding_outpoint0 =
        generate_stub_outpoint(&config.client_0, &funding_utxo_address0, input_value0).await;

    let mut kick_off_2_tx = KickOff2Transaction::new(
        &config.operator_context,
        &config.connector_1,
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

    // TODO: check whether the block hash matches the actual block No. 866191 hash
    let header = Header {
        version: Version::from_consensus(0x200d2000),
        prev_blockhash: BlockHash::from_str(
            "000000000000000000027c9f5b07f21e39ba31aa4d900d519478bdac32f4a15d",
        )
        .unwrap(),
        merkle_root: TxMerkleNode::from_str(
            "0064b0d54f20412756ba7ce07b0594f3548b06f2dad5cfeaac2aca508634ed19",
        )
        .unwrap(),
        time: 1729244761,
        bits: CompactTarget::from_hex("0x17030ecd").unwrap(),
        nonce: 0x400e345c,
    };
    let block_hash = header.block_hash();

    kick_off_2_tx.sign_input_0(
        &config.operator_context,
        &config.connector_1,
        &config.connector_1_winternitz_secrets[&0],
        &get_superblock_message(&sb, &sb_hash),
    );

    let tx = kick_off_2_tx.finalize();
    // println!("Script Path Spend Transaction: {:?}\n", tx);
    let result = config.client_0.esplora.broadcast(&tx).await;
    println!("Txid: {:?}", tx.compute_txid());
    println!("Broadcast result: {:?}\n", result);
    // println!("Transaction hex: \n{}", serialize_hex(&tx));
    assert!(result.is_ok());
}
