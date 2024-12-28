use polkadot_sdk::{
    sp_consensus::Error as ConsensusError,
    sp_runtime::traits::{Block as BlockT, Header as HeaderT},
};
use async_trait::async_trait;

#[async_trait]
pub trait BlockProducer<Block>
where
    Block: BlockT,
    Block::Header: HeaderT,
{
    async fn produce_block(
        &self,
        parent_hash: Block::Hash,
        parent_number: <<Block as BlockT>::Header as HeaderT>::Number,
    ) -> Result<Block, ConsensusError>;
} 
