use anyhow::{anyhow, ensure, Result};
use std::error::Error;
use std::fmt::Debug;

use random_access_storage::RandomAccess;
use crate::block::{Block, BLOCK_LENGTH};

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct StoreBlocks<T>
where
    T: Debug,
{
    store: T,
}
impl<T> StoreBlocks<T>
where
    T: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Create a new [StoreBlocks] from [RandomAccess] interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }

    /// Write a `Block`.
    #[inline]
    pub async fn write(
        &mut self,
        index: u32,
        block: &Block,
        ) -> Result<()>
    {
        let offset: u64 = (index as u64) * (BLOCK_LENGTH as u64);
        let data = block.to_bytes()?;
        ensure!(data.len() == BLOCK_LENGTH as usize);

        self.store
            .write(offset, &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read a `Block`.
    #[inline]
    pub async fn read(
        &mut self,
        index: u32,
        ) -> Result<Block>
    {
        let offset: u64 = (index as u64) * (BLOCK_LENGTH as u64);
        ensure!(offset + BLOCK_LENGTH as u64 <= u64::MAX);

        let data = self.store
            .read(offset, BLOCK_LENGTH as u64)
            .await.map_err(|e| anyhow!(e))?;
        Block::from_bytes(&data)
    }
}

#[cfg(test)]
mod tests {
    use async_std::test;
    use random_access_memory::RandomAccessMemory;
    use crate::block::{Signature, BlockSignature, SIGNATURE_LENGTH};
    use super::*;

    fn ram() -> RandomAccessMemory {
        let page_size = 1024;
        RandomAccessMemory::new(page_size)
    }

    #[test]
    pub async fn init() -> Result<()> {
        StoreBlocks::new(ram());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreBlocks::new(ram());
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let block = Block::new(1, 8, signature);
        store.write(0, &block).await?;
        let block2 = store.read(0).await?;
        assert_eq!(block, block2);
        Ok(())
    }
}
