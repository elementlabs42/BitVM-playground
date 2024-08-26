use bitcoin::Network;

pub fn num_blocks_per_network(network: Network, num_blocks: u32) -> u32 {
    if network == Network::Bitcoin {
        num_blocks
    } else {
        1
    }
}
