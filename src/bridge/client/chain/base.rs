use async_trait::async_trait;

use super::chain::PegOutEvent;

#[async_trait]
pub trait ChainAdaptor {
    async fn get_peg_out_init_event(&self) -> Result<Vec<PegOutEvent>, String>;
}
