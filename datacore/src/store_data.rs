use anyhow::{anyhow, ensure, Result};
use std::error::Error;
use std::fmt::Debug;

use random_access_storage::RandomAccess;
use crate::block::Block;

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct StoreData<T>
where
    T: Debug,
{
    store: T,
}
impl<T> StoreData<T>
where
    T: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Create a new [StoreData] from [RandomAccess] interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }

    /// Write data for a `Block`.
    #[inline]
    pub async fn write(
        &mut self,
        node: &Block,
        data: &[u8],
        ) -> Result<()>
    {
        let (offset, length) = verify_span(block_to_span(&node))?;
        ensure!(data.len() == length as usize);

        self.store
            .write(offset as u64, &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read data for a `Block`.
    #[inline]
    pub async fn read(
        &mut self,
        node: &Block,
        ) -> Result<Vec<u8>>
    {
        let (offset, length) = verify_span(block_to_span(&node))?;

        self.store
            .read(offset, length as u64)
            .await.map_err(|e| anyhow!(e))
    }
}

#[inline]
fn block_to_span(block: &Block) -> (u64, u32) {
    (block.offset(), block.length())
}
#[inline]
fn verify_span(span: (u64, u32)) -> Result<(u64, u32)> {
    let (offset, length) = span;
    ensure!(offset + length as u64 <= u64::MAX);
    Ok(span)
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
        StoreData::new(ram());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreData::new(ram());
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let msg = "hello world".as_bytes();
        let block = Block::new(1, msg.len() as u32, signature);
        store.write(&block, msg).await?;
        let msg2 = store.read(&block).await?;
        assert_eq!(msg, msg2);
        Ok(())
    }
}
