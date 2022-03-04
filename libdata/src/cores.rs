use std::fmt::Debug;
use std::error::Error;
use std::collections::HashMap;
use async_std::sync::{Arc, Mutex, Weak};

use crate::{
    RandomAccess, Core,
    PublicKey, DiscoveryKey, discovery_key
};

type PublicKeyBytes = [u8; 32];

/// [Cores] is a container for storing and quickly accessing multiple [Core]s.
///
/// Stored [Core]s can be accessed by [PublicKey] or [DiscoveryKey].
#[derive(Debug)]
pub struct Cores<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    by_public:    HashMap<PublicKeyBytes, Arc<Mutex<Core<D, B, M>>>>,
    by_discovery: HashMap<DiscoveryKey,  Weak<Mutex<Core<D, B, M>>>>,
}

impl<D, B, M> Cores<D, B, M>
where
    D: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    B: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
    M: RandomAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Create a new [Cores].
    #[inline]
    pub fn new() -> Self {
        Self {
            by_public: HashMap::new(),
            by_discovery: HashMap::new(),
        }
    }

    /// Insert a new [Core].
    #[inline]
    pub fn insert(&mut self, core: Core<D, B, M>)
    {
        let public = core.public_key().clone();
        let core = Arc::new(Mutex::new(core));

        self.put(&public, core);
    }
    /// Put a [Arc<Mutex<Core>>] under [PublicKey].
    pub fn put(&mut self, public: &PublicKey, core: Arc<Mutex<Core<D, B, M>>>)
    {
        let public = public.to_bytes();
        let discovery = discovery_key(&public);

        self.by_discovery.insert(discovery, Arc::downgrade(&core));
        self.by_public.insert(public, core);
    }

    /// Try getting a [Core] by [PublicKey].
    #[inline]
    pub fn get_by_public(&self, key: &PublicKey)
        -> Option<Arc<Mutex<Core<D, B, M>>>>
    {
        self.by_public.get(&key.to_bytes())
            .map(Arc::clone)
    }

    /// Try getting a [Core] by [DiscoveryKey].
    #[inline]
    pub fn get_by_discovery(&self, key: &DiscoveryKey)
        -> Option<Arc<Mutex<Core<D, B, M>>>>
    {
        self.by_discovery.get(key)
            .map(|weak| weak.upgrade())
            .flatten()
    }

    /// Returns the number of contained [Core]s.
    #[inline]
    pub fn len(&self) -> usize {
        self.by_public.len()
    }

    /// Get the [PublicKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn public_keys(&self) -> Vec<PublicKey>
    {
        self.by_public
            .keys()
            .map(|bytes| PublicKey::from_bytes(bytes).unwrap())
            .collect()
    }

    /// Get the [DiscoveryKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn discovery_keys(&self) -> Vec<DiscoveryKey>
    {
        self.by_public
            .keys()
            .map(|bytes| discovery_key(bytes))
            .collect()
    }

    /// Access the contained [Core]s.
    #[inline]
    pub fn entries(&self)
        ->  Vec<(PublicKey, Arc<Mutex<Core<D, B, M>>>)>
    {
        self.by_public
            .iter()
            .map(|(bytes, core)|
                 (PublicKey::from_bytes(bytes).unwrap(), Arc::clone(core)))
            .collect()
    }
}
