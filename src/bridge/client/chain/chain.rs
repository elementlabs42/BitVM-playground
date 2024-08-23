use bitcoin::{OutPoint, Txid};
use std::str::FromStr;

use super::{base::ChainAdaptor, ethereum::EthereumAdaptor};

pub struct PegOutEvent {
    pub source_outpoint: OutPoint,
}

impl PegOutEvent {
    pub const EVENT_NAME: &'static str =
        "PegOutInitiated(address,string,(bytes32,uint256),uint256,bytes32)";
}

static CLIENT_MISSING_ORACLE_DRIVER_ERROR: &str = "Bridge client is missing chain adaptor";

pub struct Chain {
    ethereum: Option<EthereumAdaptor>,
}

impl Chain {
    pub fn new() -> Self {
        Self {
            ethereum: EthereumAdaptor::new(),
        }
    }

    pub async fn get_peg_out_init(&self) -> Result<PegOutEvent, String> {
        match self.get_driver() {
            Ok(driver) => match driver.get_peg_out_init_event().await {
                Ok(keys) => Ok(PegOutEvent {
                    source_outpoint: OutPoint {
                        txid: Txid::from_str(
                            "4e254eab8a41f14f56491813a7100cebe305d84edf09488001d9dd3d180a4900",
                        )
                        .unwrap(),
                        vout: 0,
                    },
                }),
                Err(err) => Err(err.to_string()),
            },
            Err(err) => Err(err.to_string()),
        }
    }

    fn get_driver(&self) -> Result<&dyn ChainAdaptor, &str> {
        if self.ethereum.is_some() {
            return Ok(self.ethereum.as_ref().unwrap());
        } else {
            Err(CLIENT_MISSING_ORACLE_DRIVER_ERROR)
        }
    }
}
