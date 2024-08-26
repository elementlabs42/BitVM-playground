use async_trait::async_trait;

use super::chain::PegOutEvent;
use super::chain::PegInMintedEvent;

#[async_trait]
pub trait ChainAdaptor {
    async fn get_peg_out_init_event(&self) -> Result<Vec<PegOutEvent>, String>;
    async fn get_peg_in_minted_event(&self) -> Result<Vec<PegInMintedEvent>, String>;
}
