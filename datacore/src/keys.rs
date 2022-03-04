//! Generate a `Keypair`, sign and verify messages with `Keypair`.
//! Uses `Ed25519` cryptography.

use anyhow::{Result, ensure};
use rand::rngs::{OsRng, StdRng};
use rand::SeedableRng;
use ed25519_dalek::{ExpandedSecretKey, Verifier};

pub use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};

/// Create a new [Keypair].
pub fn generate_keypair() -> Keypair {
    let mut rng = StdRng::from_rng(OsRng::default()).unwrap();
    Keypair::generate(&mut rng)
}

/// Sign a byte slice.
pub fn sign(public: &PublicKey, secret: &SecretKey, msg: &[u8]) -> Signature {
    ExpandedSecretKey::from(secret).sign(msg, public)
}

/// Verify a signature of a byte slice.
pub fn verify(
    public: &PublicKey,
    msg: &[u8],
    signature: &Signature,
    ) -> Result<()>
{
    ensure!(public.verify(msg, signature).is_ok(), "Signature invalid.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify() {
        let keypair = generate_keypair();
        let msg = b"hello";
        let signature = sign(&keypair.public, &keypair.secret, msg);
        assert!(verify(&keypair.public, msg, &signature).is_ok());
        assert!(verify(&keypair.public, b"oops", &signature).is_err());
    }
}
