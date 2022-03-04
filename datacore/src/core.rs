//! Main `Core` abstraction.
//! Exposes an append-only, single-writer, secure log structure.

use anyhow::{Result, ensure, bail};
use std::error::Error;
use std::fmt::Debug;
use futures_lite::future::zip;

use crate::store_data::StoreData;
use crate::store_blocks::StoreBlocks;
use crate::store_state::StoreState;
use crate::merkle::{Merkle, NodeTrait};
use crate::{
    Block, BlockSignature, Hash, RandomAccess,
    PublicKey, SecretKey, sign, verify,
};

/// Maximum number of blocks of data in a `Core`.
pub const MAX_CORE_LENGTH: usize = u32::MAX as usize;
/// Maximum size of a single block of data in a `Core`.
pub const MAX_BLOCK_SIZE: usize = u32::MAX as usize;

/// Core is an append-only, single-writer, secure log structure.
///
/// To read an entry from a `Core` you only need to know its [PublicKey],
/// to write to a `Core` you must also have its [SecretKey].
/// The [SecretKey] should not be shared unless you know what you're doing
/// as only one client should be able to write to a single `Core`.
/// If 2 separate clients write conflicting information to the same `Core`
/// it will become corrupted.
///
/// The feed needs an implementation of [RandomAccess] as a storage backing
/// for the entries added to it.
///
/// [SecretKey]: ed25519_dalek::SecretKey
/// [PublicKey]: ed25519_dalek::PublicKey
/// [RandomAccess]: random_access_storage::RandomAccess
#[derive(Debug)]
pub struct Core<D, B, S>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    S: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    data: StoreData<D>,
    blocks: StoreBlocks<B>,
    state: StoreState<S>,

    merkle: Merkle,
    public_key: PublicKey,
    secret_key: Option<SecretKey>,

    length: u32,
    byte_length: u64,
}

impl<D, B, S> Core<D, B, S>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    S: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Create a new instance with a custom storage backend.
    pub async fn new(
        data: D,
        blocks: B,
        state: S,
        public_key: PublicKey,
        secret_key: Option<SecretKey>
        ) -> Result<Self>
    {
        let data = StoreData::new(data);
        let mut blocks = StoreBlocks::new(blocks);
        let mut state = StoreState::new(state);

        let merkle = state.read().await?;
        let length = merkle.blocks() as u32;
        let byte_length = match length {
            0 => 0,
            n => {
                let block = blocks.read(n - 1).await?;
                block.offset() as u64 + block.length() as u64
            },
        };

        Ok(Self {
            data,
            blocks,
            state,
            merkle,
            public_key,
            secret_key,
            length,
            byte_length,
        })
    }

    /// Get the number of entries in the `Core`.
    #[inline]
    pub fn len(&self) -> u32 {
        self.length
    }
    /// Check if the `Core` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Access the [PublicKey].
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    /// Access the optional [SecretKey].
    pub fn secret_key(&self) -> &Option<SecretKey> {
        &self.secret_key
    }

    /// Append data into the `Core`.
    ///
    /// If `signature` is supplied, the caller is responsible for verifying its
    /// integrity and consistency with the `data`.
    #[inline]
    pub async fn append(
        &mut self,
        data: &[u8],
        signature: Option<BlockSignature>,
        ) -> Result<()>
    {
        let index = self.len();
        let data_length = data.len();
        ensure!(data_length <= MAX_BLOCK_SIZE);

        // get or try to create the `signature`
        let signature = match signature {
            Some(signature) => {
                let data_hash = Hash::from_leaf(data);
                verify(&self.public_key, &data_hash, &signature.data())?;
                let mut merkle = self.merkle.clone();
                merkle.next(data_hash, data_length as u64);
                verify(&self.public_key,
                       &hash_merkle(&merkle), &signature.tree())?;
                self.merkle = merkle;
                signature
            },
            None => {
                let secret = match &self.secret_key {
                    Some(secret) => secret,
                    None => bail!("No SecretKey for Core, cannot append."),
                };
                let data_hash = Hash::from_leaf(data);
                let data_sign = sign(&self.public_key, &secret, &data_hash);
                self.merkle.next(data_hash, data_length as u64);
                let tree_sign = sign(&self.public_key,
                                     &secret, &hash_merkle(&self.merkle));
                BlockSignature::new(data_sign, tree_sign)
            },
        };

        let block = Block::new(
            self.byte_length, data_length as u32, signature);

        let (d, b) = zip(
            self.data.write(&block, &data),
            self.blocks.write(index, &block))
            .await; d?; b?;
        self.state.write(&self.merkle).await?;
        self.byte_length += data_length as u64;
        self.length += 1;

        Ok(())
    }

    /// Get the block of data at the tip of the feed.
    /// This will be the most recently appended block.
    #[inline]
    pub async fn head(&mut self)
        -> Result<Option<(Vec<u8>, BlockSignature)>>
    {
        match self.len() {
            0 => Ok(None),
            len => self.get(len - 1).await,
        }
    }
    /// Retrieve data for a block at index.
    #[inline]
    pub async fn get(&mut self, index: u32)
        -> Result<Option<(Vec<u8>, BlockSignature)>>
    {
        ensure!((index as usize) < MAX_CORE_LENGTH);
        let length = self.len();
        if index >= length {
            return Ok(None)
        }
        let block = self.blocks.read(index).await?;
        let data = self.data.read(&block).await?;
        Ok(Some((data, block.signature())))
    }
}

#[inline]
fn hash_merkle(merkle: &Merkle) -> Hash {
    let roots = merkle.roots();
    let hashes = roots.iter()
        .map(|root| root.hash())
        .collect::<Vec<&Hash>>();
    let lengths = roots.iter()
        .map(|root| root.len())
        .collect::<Vec<u64>>();
    Hash::from_roots(&hashes, &lengths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn max_sizes_fit() {
        let max_length = MAX_CORE_LENGTH * MAX_BLOCK_SIZE;
        assert!(max_length <= u64::MAX as usize);
    }
}
