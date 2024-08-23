use bitvm::bridge::client::chain::{base::ChainAdaptor, ethereum::EthereumAdaptor};

#[tokio::test]
async fn test_ethereum_rpc() {
    let adaptor = EthereumAdaptor::new().unwrap();
    let events = adaptor.get_peg_out_init_event().await;
    for event in events.unwrap() {
        println!("{:?}", event);
    }
}
