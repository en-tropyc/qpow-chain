use super::block_production::BlockProducer;
use polkadot_sdk::{
    sc_client_api::{Backend, BlockBackend, HeaderBackend},
    sp_api::{Core, ProvideRuntimeApi},
    sp_block_builder::BlockBuilder,
    sp_consensus::Error as ConsensusError,
    sp_inherents::{InherentData, InherentDataProvider},
    sp_runtime::traits::{Block as BlockT, Header as HeaderT},
    sp_timestamp,
};
use std::{marker::PhantomData, sync::Arc, time::SystemTime};
use async_trait::async_trait;

pub struct BasicBlockProducer<Block: BlockT, Client, BE> {
    client: Arc<Client>,
    _phantom: PhantomData<(Block, BE)>,
}

impl<Block, Client, BE> BasicBlockProducer<Block, Client, BE>
where
    Block: BlockT,
    Block::Header: HeaderT,
    BE: Backend<Block>,
    Client: HeaderBackend<Block> + ProvideRuntimeApi<Block> + BlockBackend<Block>,
{
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Block, Client, BE> BlockProducer<Block> for BasicBlockProducer<Block, Client, BE>
where
    Block: BlockT,
    Client: HeaderBackend<Block> 
        + ProvideRuntimeApi<Block> 
        + BlockBackend<Block> 
        + Send 
        + Sync,
    Client::Api: BlockBuilder<Block> + Core<Block>,
    BE: Backend<Block> + Send + Sync,
{
    async fn produce_block(
        &self,
        parent_hash: Block::Hash,
        _parent_number: <<Block as BlockT>::Header as HeaderT>::Number,
    ) -> Result<Block, ConsensusError> {
        // Create timestamp for the new block
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Create inherent data
        let mut inherent_data = InherentData::new();
        let timestamp_data = sp_timestamp::InherentDataProvider::new(timestamp.into());
        
        timestamp_data
            .provide_inherent_data(&mut inherent_data)
            .await
            .map_err(|e| ConsensusError::Other(Box::new(e)))?;

        // Get the runtime API
        let runtime_api = self.client.runtime_api();

        // Get the parent header
        let parent_header = self
            .client
            .header(parent_hash)
            .map_err(|e| ConsensusError::Other(Box::new(e)))?
            .ok_or(ConsensusError::ChainLookup("Parent header not found".into()))?;

        // Initialize block with at parent hash
        runtime_api
            .initialize_block(parent_hash, &parent_header)
            .map_err(|e| ConsensusError::Other(Box::new(e)))?;

        // Create inherent extrinsics
        let inherent_extrinsics = runtime_api
            .inherent_extrinsics(parent_hash, inherent_data)
            .map_err(|e| ConsensusError::Other(Box::new(e)))?;

        // Add all inherent extrinsics
        for inherent in inherent_extrinsics.iter() {
            runtime_api
                .apply_extrinsic(parent_hash, inherent.clone())
                .map_err(|e| ConsensusError::Other(Box::new(e)))?;
        }

        // Finalize the block
        let header = runtime_api
            .finalize_block(parent_hash)
            .map_err(|e| ConsensusError::Other(Box::new(e)))?;

        // Create the block with inherent extrinsics
        Ok(Block::new(header, inherent_extrinsics))
    }
} 
