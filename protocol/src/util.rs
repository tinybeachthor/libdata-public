use blake3::keyed_hash;

use crate::DiscoveryKey;

/// Seed for the discovery key hash.
const DISCOVERY_NS_BUF: &[u8] = b"hypercore";

/// Calculate the discovery key of a key.
///
/// The discovery key is a 32 byte namespaced hash of the key.
pub fn discovery_key(key: &[u8; 32]) -> DiscoveryKey {
    *keyed_hash(key, &DISCOVERY_NS_BUF).as_bytes()
}
