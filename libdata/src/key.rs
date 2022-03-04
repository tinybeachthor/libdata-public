use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{SeedableRng, RngCore};
use rand;
use blake3::derive_key;

pub use datacore::{generate_keypair, Keypair, PublicKey, SecretKey};
pub use protocol::{DiscoveryKey, discovery_key};

struct CSPRNG (ChaCha20Rng);

impl SeedableRng for CSPRNG {
    type Seed = [u8; 32];

    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        Self(ChaCha20Rng::from_seed(seed))
    }
}
impl rand::RngCore for CSPRNG {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
    #[inline]
    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.0.fill_bytes(bytes)
    }
    #[inline]
    fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), rand::Error> {
        match self.0.try_fill_bytes(bytes) {
            Err(err) => Err(rand::Error::from(err.code().unwrap())),
            Ok(()) => Ok(()),
        }
    }
}
impl rand::CryptoRng for CSPRNG {}

/// Derive a named [Keypair] from a base [SecretKey].
pub fn derive_keypair(key: &SecretKey, name: &str) -> Keypair {
    let seed: <CSPRNG as SeedableRng>::Seed =
        derive_key(name, &key.to_bytes()).into();

    let mut rng = CSPRNG::from_seed(seed);
    Keypair::generate(&mut rng)
}
