use super::basic_block_producer::BasicBlockProducer;
use super::block_production::BlockProducer;
use log::{debug, info, warn, trace};
use polkadot_sdk::{
    sc_client_api::{Backend, BlockBackend, BlockchainEvents, HeaderBackend},
    sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy},
    sp_api::{Core, ProvideRuntimeApi},
    sp_consensus::{BlockOrigin, Error as ConsensusError},
    sp_runtime::{
        traits::{Block as BlockT, One},
        Saturating,
        SaturatedConversion,
    },
    sp_block_builder::BlockBuilder,
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
    BE: Backend<Block> + Send + Sync,
    Client: BlockBackend<Block> 
        + BlockchainEvents<Block> 
        + HeaderBackend<Block> 
        + ProvideRuntimeApi<Block> 
        + Send 
        + Sync,
    Client::Api: BlockBuilder<Block> + Core<Block>,
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
        info!("🚀 Starting round-robin consensus for validator {}", self.validator_id);
        println!("🚀 Starting round-robin consensus for validator {}", self.validator_id);
        
        loop {
            if let Err(e) = self.produce_block().await {
                warn!("Failed to produce block: {:?}", e);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
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

    async fn produce_block(&mut self) -> Result<(), ConsensusError> {
        let best_header = self.client.info().best_hash;
        let best_number = self.client.info().best_number;

        // Check if it's our turn
        let slot = best_number.saturating_add(1u32.into()) % self.total_validators.into();
        
        let msg = format!("🔄 Round-Robin: Block #{} | Slot {} | Our ID {} | Total Validators {}", 
            best_number + 1u32.into(),
            slot,
            self.validator_id,
            self.total_validators
        );
        println!("{}", msg);
        info!("{}", msg);

        if slot == self.validator_id.into() {
            let msg = format!("🎯 Our turn! Validator {} producing block #{}", 
                self.validator_id,
                best_number + 1u32.into()
            );
            println!("{}", msg);
            info!("{}", msg);
            
            let block = self.block_producer
                .produce_block(best_header, best_number)
                .await?;

            println!("✅ SUCCESS: Validator {} produced block #{} ({})", 
                self.validator_id,
                best_number + 1u32.into(),
                block.hash()
            );

            // Import the block
            let (header, body) = block.deconstruct();
            let mut import_params = BlockImportParams::new(BlockOrigin::Own, header);
            import_params.body = Some(body);
            import_params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

            match self.block_import.import_block(import_params).await {
                Ok(_) => println!("📥 Block #{} successfully imported", best_number + 1u32.into()),
                Err(e) => println!("❌ Failed to import block #{}: {:?}", best_number + 1u32.into(), e),
            }
        } else {
            println!("⏳ Not our turn. Validator {} waiting (slot {} belongs to validator {})", 
                self.validator_id,
                slot,
                slot
            );
        }

        Ok(())
    }
}
