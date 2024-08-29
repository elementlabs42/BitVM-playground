use bitvm::bridge::client::chain::{base::ChainAdaptor, ethereum::EthereumAdaptor};

#[tokio::test]
async fn test_ethereum_rpc() {
    let adaptor = EthereumAdaptor::new().unwrap();
    let result = adaptor.get_peg_out_init_event().await;
    assert!(result.is_ok());

    let events = result.unwrap();
    for event in events {
        println!("{:?}", event);
    }
}
