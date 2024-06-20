#[cfg(test)]
mod tests {

    use bitcoin::{
        consensus::encode::serialize_hex, key::Keypair, Amount, Network, OutPoint, PrivateKey,
        PublicKey, TxOut,
    };

    use bitvm::bridge::{
        components::{
            bridge::BridgeTransaction,
            connector::*,
            connector_3::Connector3,
            connector_c::ConnectorC,
            disprove::*,
            helper::{generate_pay_to_pubkey_script, Input},
        },
        graph::{DUST_AMOUNT, FEE_AMOUNT, INITIAL_AMOUNT},
    };

    use crate::bridge::setup::setup_test;

    #[tokio::test]
    async fn test_should_be_able_to_submit_disprove_tx_successfully() {
        let (client, context) = setup_test();

        let connector_3 = Connector3::new(context.network, &context.n_of_n_public_key.unwrap());
        let connector_c =
            ConnectorC::new(context.network, &context.n_of_n_taproot_public_key.unwrap());

        let funding_utxo_1 = client
            .get_initial_utxo(
                connector_c.generate_taproot_address(),
                Amount::from_sat(INITIAL_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_c.generate_taproot_address(),
                    INITIAL_AMOUNT
                );
            });

        println!("funding_utxo_1.txid {}", funding_utxo_1.txid.as_raw_hash());
        println!("funding_utxo_1.value {}", funding_utxo_1.value);

        let funding_utxo_0 = client
            .get_initial_utxo(
                connector_3.generate_address(),
                Amount::from_sat(DUST_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_3.generate_address(),
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

        let mut disprove_tx = DisproveTransaction::new(
            &context,
            Input {
                outpoint: funding_outpoint_0,
                amount: Amount::from_sat(DUST_AMOUNT),
            },
            Input {
                outpoint: funding_outpoint_1,
                amount: Amount::from_sat(INITIAL_AMOUNT),
            },
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
    async fn test_should_be_able_to_submit_disprove_tx_with_verifier_added_to_output_successfully()
    {
        let (client, context) = setup_test();

        let connector_3 = Connector3::new(context.network, &context.n_of_n_public_key.unwrap());
        let connector_c =
            ConnectorC::new(context.network, &context.n_of_n_taproot_public_key.unwrap());

        let funding_utxo_1 = client
            .get_initial_utxo(
                connector_c.generate_taproot_address(),
                Amount::from_sat(INITIAL_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_c.generate_taproot_address(),
                    INITIAL_AMOUNT
                );
            });

        let funding_utxo_0 = client
            .get_initial_utxo(
                connector_3.generate_address(),
                Amount::from_sat(DUST_AMOUNT),
            )
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at https://faucet.mutinynet.com/",
                    connector_3.generate_address(),
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

        let mut disprove_tx = DisproveTransaction::new(
            &context,
            Input {
                outpoint: funding_outpoint_0,
                amount: Amount::from_sat(DUST_AMOUNT),
            },
            Input {
                outpoint: funding_outpoint_1,
                amount: Amount::from_sat(INITIAL_AMOUNT),
            },
            1,
        );

        disprove_tx.pre_sign(&context);
        let mut tx = disprove_tx.finalize(&context);

        let secp = context.secp;
        let verifier_secret: &str =
            "aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffff1234";
        let verifier_keypair = Keypair::from_seckey_str(&secp, verifier_secret).unwrap();
        let verifier_private_key = PrivateKey::new(verifier_keypair.secret_key(), context.network);
        let verifier_pubkey = PublicKey::from_private_key(&secp, &verifier_private_key);

        let verifier_output = TxOut {
            value: (Amount::from_sat(INITIAL_AMOUNT) - Amount::from_sat(FEE_AMOUNT)) / 2,
            script_pubkey: generate_pay_to_pubkey_script(&verifier_pubkey),
        };

        tx.output.push(verifier_output);

        println!("Script Path Spend Transaction: {:?}\n", tx);
        let result = client.esplora.broadcast(&tx).await;
        println!("Txid: {:?}", tx.compute_txid());
        println!("Broadcast result: {:?}\n", result);
        println!("Transaction hex: \n{}", serialize_hex(&tx));
        assert!(result.is_ok());
    }
}
