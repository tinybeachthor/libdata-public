use anyhow::Result;
use std::fmt::Debug;
use std::error::Error;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use futures_lite::stream::Stream;
use futures_lite::future::FutureExt;
use async_std::sync::{Arc, Mutex};

use crate::{RandomAccess, Core, BlockSignature};

/// Async [Stream] iterator over [Core].
pub struct CoreIterator<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    core: Arc<Mutex<Core<D, B, M>>>,
    task: Pin<Box<dyn Future<Output=(u32, Option<Vec<u8>>)>>>,
}
impl<D: 'static, B: 'static, M: 'static> CoreIterator<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    /// Create a new [CoreIterator].
    pub fn new(core: Arc<Mutex<Core<D, B, M>>>, index: u32) -> Self {
        let task = Self::create_read_task(Arc::clone(&core), index);
        Self {
            core,
            task,
        }
    }

    #[inline]
    fn create_read_task(
        core: Arc<Mutex<Core<D, B, M>>>,
        index: u32,
        ) -> Pin<Box<dyn Future<Output=(u32, Option<Vec<u8>>)>>>
    {
        async move {
            let result: Result<Option<(Vec<u8>, BlockSignature)>>;
            {
                let mut core = core.lock().await;
                result = core.get(index).await;
            }
            if let Ok(Some(data)) = result {
                (index, Some(data.0))
            }
            else {
                (index, None)
            }
        }.boxed()
    }
}
impl<D: 'static, B: 'static, M: 'static> Stream for CoreIterator<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    type Item = (u32, Vec<u8>);

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>>
    {
        let this = self.get_mut();
        if let Poll::Ready((index, data)) = Pin::new(&mut this.task).poll(cx) {
            this.task = Self::create_read_task(
                Arc::clone(&this.core), index + 1);
            return Poll::Ready(data.map(|data| (index, data)))
        }
        Poll::Pending
    }
}
impl<D: 'static, B: 'static, M: 'static> Debug for CoreIterator<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Send + Debug,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        write!(fmt, "CoreIterator")
    }
}
