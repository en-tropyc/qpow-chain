use super::basic_block_producer::BasicBlockProducer;
use super::block_production::BlockProducer;
use log::{debug, info, warn};
use polkadot_sdk::{
    sc_client_api::{Backend, BlockBackend, BlockchainEvents, HeaderBackend},
    sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy},
    sp_api::ProvideRuntimeApi,
    sp_consensus::{BlockOrigin, Error as ConsensusError},
    sp_runtime::{
        traits::{Block as BlockT, One},
        SaturatedConversion,
    },
};
use std::{sync::Arc, time::Duration};

pub struct RoundRobinConsensus<Block: BlockT, Client, BE> {
    client: Arc<Client>,
    block_import: Box<dyn BlockImport<Block, Error = ConsensusError> + Send>,
    block_producer: BasicBlockProducer<Block, Client, BE>,
    validator_id: u32,
    total_validators: u32,
}

impl<Block, Client, BE> RoundRobinConsensus<Block, Client, BE>
where
    Block: BlockT,
    BE: Backend<Block>,
    Client: BlockBackend<Block> 
        + BlockchainEvents<Block> 
        + HeaderBackend<Block> 
        + ProvideRuntimeApi<Block>,
{
    pub fn new(
        client: Arc<Client>,
        block_import: Box<dyn BlockImport<Block, Error = ConsensusError> + Send>,
        validator_id: u32,
        total_validators: u32,
    ) -> Self {
        let block_producer = BasicBlockProducer::new(client.clone());
        Self {
            client,
            block_import,
            block_producer,
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
        let best_hash = info.best_hash;
        let best_number = info.best_number;
        
        let next_validator = (best_number.saturated_into::<u32>() + 1) % self.total_validators;
        
        if next_validator != self.validator_id {
            debug!(target: "round-robin", "Not our turn to produce block");
            return Ok(());
        }

        info!(
            target: "round-robin", 
            "Building block {}", 
            (best_number + One::one()).saturated_into::<u32>()
        );

        // Produce the new block
        let block = self.block_producer
            .produce_block(best_hash, best_number)
            .await?;

        // Import the produced block
        let (header, body) = block.deconstruct();
        let mut import_params = BlockImportParams::new(BlockOrigin::Own, header);
        import_params.body = Some(body);
        import_params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

        self.block_import
            .import_block(import_params)
            .await
            .map_err(|e| ConsensusError::Other(Box::new(e)))?;

        info!(
            target: "round-robin",
            "Validator {} produced block at height {}",
            self.validator_id,
            (best_number + One::one()).saturated_into::<u32>()
        );

        Ok(())
    }
}
