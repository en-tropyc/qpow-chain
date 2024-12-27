use log::{debug, info, warn};
use polkadot_sdk::{
    sc_client_api::{BlockBackend, BlockchainEvents, HeaderBackend},
    sc_consensus::BlockImport,
    sp_consensus::Error as ConsensusError,
    sp_runtime::traits::Block as BlockT,
};
use std::{sync::Arc, time::Duration};

pub struct RoundRobinConsensus<Block: BlockT, Client> {
    client: Arc<Client>,
    _block_import: Box<dyn BlockImport<Block, Error = ConsensusError> + Send>,
    validator_id: u32,
    total_validators: u32,
}

impl<Block, Client> RoundRobinConsensus<Block, Client>
where
    Block: BlockT,
    Client: BlockBackend<Block> + BlockchainEvents<Block> + HeaderBackend<Block>,
{
    pub fn new(
        client: Arc<Client>,
        block_import: Box<dyn BlockImport<Block, Error = ConsensusError> + Send>,
        validator_id: u32,
        total_validators: u32,
    ) -> Self {
        Self {
            client,
            _block_import: block_import,
            validator_id,
            total_validators,
        }
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(6));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.try_build_block().await {
                warn!(target: "round-robin", "Failed to build block: {:?}", e);
            }
        }
    }

    async fn try_build_block(&mut self) -> Result<(), ConsensusError> {
        let info = self.client.info();
        let _best_header = info.best_hash;
        let best_number: u32 = info.best_number.try_into().unwrap_or(0);
        
        let next_validator = (best_number + 1) % self.total_validators;
        
        if next_validator != self.validator_id {
            debug!(target: "round-robin", "Not our turn to produce block");
            return Ok(());
        }

        info!(target: "round-robin", "Building block {}", best_number + 1);

        info!(target: "round-robin", 
            "Validator {} producing block at height {}", 
            self.validator_id, 
            best_number + 1
        );

        Ok(())
    }
}
