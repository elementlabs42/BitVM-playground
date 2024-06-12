#[cfg(test)]
mod tests {

    use bitcoin::{
        key::{Keypair, Secp256k1},
        Amount, OutPoint,
        TxOut
    };

    use crate::bridge::{
        client::BitVMClient, 
        components::{assert::AssertTransaction, bridge::BridgeTransaction, 
        connector_b::connector_b_address, 
        connector_c::{connector_c_bounty_address, connector_c_commit_address}, 
        disprove::DisproveTransaction}, 
        context::BridgeContext, 
        graph::{DUST_AMOUNT, INITIAL_AMOUNT, N_OF_N_SECRET, ONE_HUNDRED, OPERATOR_SECRET, UNSPENDABLE_PUBKEY}
    };

    use bitcoin::consensus::encode::serialize_hex;

    #[tokio::test]
    async fn test_disprove_tx() {
        let secp = Secp256k1::new();
        let operator_key = Keypair::from_seckey_str(&secp, OPERATOR_SECRET).unwrap();
        let n_of_n_key = Keypair::from_seckey_str(&secp, N_OF_N_SECRET).unwrap();
        let client = BitVMClient::new();
        let funding_utxo_1 = client
            .get_initial_utxo(
                connector_c_commit_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                Amount::from_sat(INITIAL_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_c_commit_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                    INITIAL_AMOUNT
                );
            });
        let funding_utxo_0 = client
            .get_initial_utxo(
                connector_c_bounty_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                Amount::from_sat(DUST_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_c_bounty_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                    DUST_AMOUNT
                );
            });
        let funding_outpoint_0 = OutPoint {
            txid: funding_utxo_0.txid,
            vout: funding_utxo_0.vout,
        };
        let funding_outpoint_1 = OutPoint {
            txid: funding_utxo_1.txid,
            vout: funding_utxo_1.vout,
        };
        let prev_tx_out_1 = TxOut {
            value: Amount::from_sat(INITIAL_AMOUNT),
            script_pubkey: connector_c_commit_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0).script_pubkey(),
        };
        let prev_tx_out_0 = TxOut {
            value: Amount::from_sat(DUST_AMOUNT),
            script_pubkey: connector_c_bounty_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0)
                .script_pubkey(),
        };
        let mut context = BridgeContext::new();
        context.set_operator_key(operator_key);
        context.set_n_of_n_pubkey(n_of_n_key.x_only_public_key().0);
        context.set_unspendable_pubkey(*UNSPENDABLE_PUBKEY);

        let mut disprove_tx = DisproveTransaction::new(
            &context,
            funding_outpoint_1,
            funding_outpoint_0,
            Amount::from_sat(INITIAL_AMOUNT),
            Amount::from_sat(DUST_AMOUNT),
            1,
        );

        disprove_tx.pre_sign(&context);
        let tx = disprove_tx.finalize(&context);
        println!("Script Path Spend Transaction: {:?}\n", tx);
        let result = client.esplora.broadcast(&tx).await;
        println!("Txid: {:?}", tx.compute_txid());
        println!("Broadcast result: {:?}\n", result);
        println!("Transaction hex: \n{}", serialize_hex(&tx));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assert_tx() {
        let secp = Secp256k1::new();
        let n_of_n_key = Keypair::from_seckey_str(&secp, N_OF_N_SECRET).unwrap();
        let operator_key = Keypair::from_seckey_str(&secp, OPERATOR_SECRET).unwrap();
        let client = BitVMClient::new();
        let input_value = Amount::from_sat(ONE_HUNDRED * 2 / 100);
        let funding_utxo = client
            .get_initial_utxo(
                connector_b_address(n_of_n_key.x_only_public_key().0, operator_key.x_only_public_key().0),
                input_value,
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_b_address(n_of_n_key.x_only_public_key().0, operator_key.x_only_public_key().0),
                    input_value.to_sat()
                );
            });
        let funding_outpoint = OutPoint {
            txid: funding_utxo.txid,
            vout: funding_utxo.vout,
        };
        let funding_utxo_pre_sign = client
            .get_initial_utxo(
                connector_c_bounty_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                Amount::from_sat(DUST_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_c_bounty_address(operator_key.x_only_public_key().0, n_of_n_key.x_only_public_key().0),
                    DUST_AMOUNT
                );
            });
        let funding_outpoint_pre_sign = OutPoint {
            txid: funding_utxo_pre_sign.txid,
            vout: funding_utxo_pre_sign.vout,
        };
        let mut context = BridgeContext::new();
        context.set_n_of_n_pubkey(n_of_n_key.x_only_public_key().0);
        context.set_operator_key(operator_key);

        let mut assert_tx = AssertTransaction::new(
            &context,
            funding_outpoint,
            input_value,
            funding_outpoint_pre_sign,
            Amount::from_sat(DUST_AMOUNT),
            1
        );

        assert_tx.pre_sign(&context);
        let tx = assert_tx.finalize(&context);
        println!("Script Path Spend Transaction: {:?}\n", tx);
        let result = client.esplora.broadcast(&tx).await;
        println!("Txid: {:?}", tx.compute_txid());
        println!("Broadcast result: {:?}\n", result);
        println!("Transaction hex: \n{}", serialize_hex(&tx));
        assert!(result.is_ok());
    }
}
