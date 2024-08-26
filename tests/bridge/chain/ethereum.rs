use bitvm::bridge::{
    client::chain::{base::ChainAdaptor, ethereum::EthereumAdaptor},
    transactions::assert,
};

#[tokio::test]
async fn test_ethereum_rpc() {
    let adaptor = EthereumAdaptor::new().unwrap();
    let result = adaptor.get_peg_out_init_event().await;
    println!("result: {:?}", result);
    let events = result.unwrap();
    for event in events {
        println!("{:?}", event);
    }
}
