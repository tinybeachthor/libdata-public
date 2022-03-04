#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! Continuously read/write to disk, using random offsets and lengths.

use anyhow::{anyhow, Error};
use async_std::fs::{self, OpenOptions};
use async_std::io::prelude::{SeekExt, WriteExt};
use async_std::io::{ReadExt, SeekFrom};
use random_access_storage::RandomAccess;
use std::ops::Drop;
use std::path::PathBuf;

/// Main constructor.
#[derive(Debug)]
pub struct RandomAccessDisk {
    file: Option<fs::File>,
    length: u64,
}

impl RandomAccessDisk {
    /// Create a new instance.
    #[allow(clippy::new_ret_no_self)]
    pub async fn open(filename: PathBuf) -> Result<RandomAccessDisk, Error>
    {
        if let Some(dirname) = filename.parent() {
            mkdirp::mkdirp(&dirname)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&filename)
            .await?;
        file.sync_all().await?;

        let metadata = filename.metadata()?;
        Ok(RandomAccessDisk {
            file: Some(file),
            length: metadata.len(),
        })
    }
}

#[async_trait::async_trait]
impl RandomAccess for RandomAccessDisk {
    type Error = Box<dyn std::error::Error + Sync + Send>;

    async fn write(
        &mut self,
        offset: u64,
        data: &[u8],
        ) -> Result<(), Self::Error> {
        let mut file = self.file.as_ref().expect("self.file was None.");
        file.seek(SeekFrom::Start(offset)).await?;
        file.write_all(&data).await?;
        file.sync_all().await?;

        // We've changed the length of our file.
        let new_len = offset + (data.len() as u64);
        if new_len > self.length {
            self.length = new_len;
        }

        Ok(())
    }

    // NOTE(yw): disabling clippy here because we files on disk might be sparse,
    // and sometimes you might want to read a bit of memory to check if it's
    // formatted or not. Returning zeroed out memory seems like an OK thing to do.
    // We should probably come back to this at a future point, and determine
    // whether it's okay to return a fully zeroed out slice. It's a bit weird,
    // because we're replacing empty data with actual zeroes - which does not
    // reflect the state of the world.
    // #[cfg_attr(test, allow(unused_io_amount))]
    async fn read(
        &mut self,
        offset: u64,
        length: u64,
        ) -> Result<Vec<u8>, Self::Error> {
        if (offset + length) as u64 > self.length {
            return Err(
                anyhow!(
                    "Read bounds exceeded. {} < {}..{}",
                    self.length,
                    offset,
                    offset + length
                    )
                .into(),
                );
        }

        let mut file = self.file.as_ref().expect("self.file was None.");
        let mut buffer = vec![0; length as usize];
        file.seek(SeekFrom::Start(offset)).await?;
        let _bytes_read = file.read(&mut buffer[..]).await?;
        Ok(buffer)
    }
}

impl Drop for RandomAccessDisk {
    fn drop(&mut self) {
        if let Some(file) = &self.file {
            // We need to flush the file on drop. Unfortunately, that is not possible to do in a
            // non-blocking fashion, but our only other option here is losing data remaining in the
            // write cache. Good task schedulers should be resilient to occasional blocking hiccups in
            // file destructors so we don't expect this to be a common problem in practice.
            // (from async_std::fs::File::drop)
            let _ = async_std::task::block_on(file.sync_all());
        }
    }
}
