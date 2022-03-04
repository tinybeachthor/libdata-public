use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::error::Error;
use std::fmt::Debug;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use random_access_storage::RandomAccess;
use crate::merkle::{Merkle, Node, NODE_SIZE};

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct StoreState<T>
where
    T: Debug,
{
    store: T,
}
impl<T> StoreState<T>
where
    T: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Create a new [StoreState] from [RandomAccess] interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }

    /// Write `Merkle` roots.
    #[inline]
    pub async fn write(
        &mut self,
        merkle: &Merkle,
        ) -> Result<()>
    {
        let roots = merkle.roots();
        let length = roots.len() as u32;

        let mut data = Vec::with_capacity(
            size_of::<u32>() + length as usize * NODE_SIZE);
        data.write_u32::<LittleEndian>(length)?;
        for node in roots {
            data.extend_from_slice(&node.to_bytes()?);
        }

        self.store
            .write(0, &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read roots and reconstruct `Merkle`.
    #[inline]
    pub async fn read(
        &mut self,
        ) -> Result<Merkle>
    {
        // try reading length
        let read_header = self.store
            .read(0, size_of::<u32>() as u64)
            .await.map_err(|e| anyhow!(e));

        // init [Merkle] from roots
        let roots = match read_header {
            // no length => no roots
            Err(_) => vec![],
            // read roots
            Ok(header) => {
                let length = Cursor::new(header).read_u32::<LittleEndian>()?;

                let mut roots = Vec::with_capacity(
                    length as usize * size_of::<Node>());
                let data = self.store
                    .read(
                        size_of::<u32>() as u64,
                        length as u64 * NODE_SIZE as u64)
                    .await.map_err(|e| anyhow!(e))?;

                let mut start = 0;
                while start < data.len() {
                    let end = start + NODE_SIZE;
                    let root = Node::from_bytes(&data[start..end])?;
                    roots.push(root);
                    start = end;
                }
                roots
            },
        };
        Ok(Merkle::from_roots(roots))
    }
}

#[cfg(test)]
mod tests {
    use async_std::test;
    use random_access_memory::RandomAccessMemory;
    use crate::hash::Hash;
    use super::*;

    fn ram() -> RandomAccessMemory {
        let page_size = 1024;
        RandomAccessMemory::new(page_size)
    }

    #[test]
    pub async fn init() -> Result<()> {
        StoreState::new(ram());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreState::new(ram());
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf(b"a"), 1);
        merkle.next(Hash::from_leaf(b"b"), 1);
        merkle.next(Hash::from_leaf(b"c"), 1);
        store.write(&merkle).await?;
        let merkle2 = store.read().await?;
        assert_eq!(merkle.roots(), merkle2.roots());
        Ok(())
    }
}
