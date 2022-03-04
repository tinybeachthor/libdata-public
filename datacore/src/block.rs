use anyhow::Result;
use std::mem::size_of;
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub use ed25519_dalek::{Signature, SIGNATURE_LENGTH};

/// [BlockSignature] holds [Signature]s - `data` and `tree` - for a [Block].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BlockSignature {
    data: Signature,
    tree: Signature,
}

impl BlockSignature {
    /// Create a new [BlockSignature].
    #[inline]
    pub fn new(data: Signature, tree: Signature) -> Self {
        Self {
            data,
            tree,
        }
    }

    /// Get data [Signature].
    pub fn data(&self) -> Signature {
        self.data
    }

    /// Get tree [Signature].
    pub fn tree(&self) -> Signature {
        self.tree
    }
}

/// [Block] describes a block of data in `Core`.
/// Includes offset and length of the content data.
/// Includes data signature verifying the data content and
/// a tree signature verifying the block position in the `Core`.
#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    offset: u64,
    length: u32,
    signature: BlockSignature,
}

pub const BLOCK_LENGTH: usize
    = size_of::<u64>() + size_of::<u32>() + (2 * SIGNATURE_LENGTH);

impl Block {
    /// Create a new [Block].
    #[inline]
    pub fn new(offset: u64, length: u32, signature: BlockSignature) -> Self {
        Self {
            offset,
            length,
            signature,
        }
    }

    /// Serialize [Block].
    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(BLOCK_LENGTH);

        data.write_u64::<LittleEndian>(self.offset)?;
        data.write_u32::<LittleEndian>(self.length)?;
        data.extend_from_slice(&self.signature.data.to_bytes());
        data.extend_from_slice(&self.signature.tree.to_bytes());

        Ok(data)
    }
    /// Deserialize [Block].
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut rdr = Cursor::new(data);
        let offset = rdr.read_u64::<LittleEndian>()?;
        let length = rdr.read_u32::<LittleEndian>()?;

        let mut data_signature = [0u8; SIGNATURE_LENGTH];
        rdr.read_exact(&mut data_signature)?;
        let mut tree_signature = [0u8; SIGNATURE_LENGTH];
        rdr.read_exact(&mut tree_signature)?;

        let signature = BlockSignature::new(
            Signature::from_bytes(&data_signature)?,
            Signature::from_bytes(&tree_signature)?,
        );

        Ok(Self {
            offset,
            length,
            signature,
        })
    }

    /// Get the offset of the content of this [Block].
    #[inline]
    pub fn offset(&self) -> u64 {
        self.offset
    }
    /// Get the length of content of this [Block].
    #[inline]
    pub fn length(&self) -> u32 {
        self.length
    }
    /// Get the [BlockSignature] of this [Block].
    #[inline]
    pub fn signature(&self) -> BlockSignature {
        self.signature.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn to_bytes_from_bytes() -> Result<()> {
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let block = Block::new(1, 8, signature);
        let block2 = Block::from_bytes(&block.to_bytes()?)?;
        assert_eq!(block2, block);
        Ok(())
    }
    #[test]
    pub fn from_bytes_fails_on_incomplete_input() -> Result<()> {
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let block = Block::new(1, 8, signature);
        let result = Block::from_bytes(&block.to_bytes()?[1..]);
        assert!(result.is_err());
        Ok(())
    }
    #[test]
    pub fn get_signatures() -> Result<()> {
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        assert_eq!(signature.data(), data);
        assert_eq!(signature.tree(), tree);
        Ok(())
    }
}
