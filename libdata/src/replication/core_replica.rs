use anyhow::{Result, anyhow};
use std::error::Error;
use std::fmt::Debug;
use async_trait::async_trait;
use async_std::sync::{Arc, Mutex};

use crate::{RandomAccess, Core, BlockSignature, Signature, MAX_CORE_LENGTH};
use crate::replication::{ReplicaTrait, Request, Data, DataOrRequest};

/// CoreReplica describes eager, full, and sequential synchronization logic
/// for [Core] over [Replication].
///
/// [Replication]: super::Replication
#[derive(Debug)]
pub struct CoreReplica<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    core: Arc<Mutex<Core<D, B, M>>>,
    remote_index: Option<u32>,
}

impl<D, B, M> CoreReplica<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    /// Create a new [CoreReplica].
    pub fn new(core: Arc<Mutex<Core<D, B, M>>>) -> Self {
        Self {
            core,
            remote_index: None,
        }
    }

    fn update_remote_index(&mut self, index: u32) {
        if let Some(old_index) = self.remote_index {
            if index <= old_index {
                return
            }
        }
        self.remote_index = Some(index);
    }
}

#[async_trait]
impl<D, B, M> ReplicaTrait for CoreReplica<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    async fn on_open(&mut self) -> Result<Option<Request>> {
        let core = self.core.lock().await;
        let request = Request {
            index: core.len(),
        };
        Ok(Some(request))
    }
    async fn on_request(&mut self, request: Request)
        -> Result<Option<DataOrRequest>>
    {
        self.update_remote_index(request.index);

        let mut core = self.core.lock().await;
        let data = core.get(request.index).await?;
        Ok(match data {
            Some((data, signature)) => {
                let response = Data {
                    index: request.index,
                    data,
                    data_signature: signature.data().to_bytes().to_vec(),
                    tree_signature: signature.tree().to_bytes().to_vec(),
                };
                Some(DataOrRequest::Data(response))
            },
            None => {
                let index = core.len();
                let remote_index = self.remote_index.unwrap_or(0);
                if index as usize >= MAX_CORE_LENGTH || remote_index <= index {
                    None
                }
                else {
                    let response = Request { index };
                    Some(DataOrRequest::Request(response))
                }
            },
        })
    }
    async fn on_data(&mut self, data: Data)
        -> Result<Option<Request>>
    {
        let mut core = self.core.lock().await;
        let len = core.len();
        if data.index == len {
            let signature = BlockSignature::new(
                Signature::from_bytes(&data.data_signature).unwrap(),
                Signature::from_bytes(&data.tree_signature).unwrap());
            core.append(&data.data, Some(signature)).await?;

            if core.len() as usize >= MAX_CORE_LENGTH {
                Ok(None)
            }
            else {
                Ok(Some(Request {
                    index: data.index + 1,
                }))
            }
        }
        else {
            Ok(Some(Request {
                index: len,
            }))
        }
    }
    async fn on_close(&mut self) -> Result<()> {
        if let Some(index) = self.remote_index {
            let core = self.core.lock().await;
            let len = core.len();

            if len < index {
                return Err(anyhow!("Not synced; remote has more data."))
            }
        }
        Ok(())
    }
}
