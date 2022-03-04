use quickcheck::{quickcheck, TestResult};
use insta;

use libdata::{generate_keypair, derive_keypair, SecretKey};

#[test]
fn key_can_generate() {
    generate_keypair();
}

#[test]
fn key_can_derive() {
    let keypair = generate_keypair();
    derive_keypair(&keypair.secret, "hello");
}

quickcheck! {
    fn key_same_key_different_names(a: String, b: String) -> TestResult {
        if a == b {
            return TestResult::discard()
        }

        let main = generate_keypair();
        let a = derive_keypair(&main.secret, &a);
        let b = derive_keypair(&main.secret, &b);

        TestResult::from_bool(a.to_bytes() != b.to_bytes())
    }

    fn key_different_key_same_name(name: String) -> bool {
        let a = generate_keypair();
        let b = generate_keypair();
        let a = derive_keypair(&a.secret, &name);
        let b = derive_keypair(&b.secret, &name);

        a.to_bytes() != b.to_bytes()
    }

    fn key_same_key_same_name(name: String) -> bool {
        let main = generate_keypair();
        let a = derive_keypair(&main.secret, &name);
        let b = derive_keypair(&main.secret, &name);

        a.to_bytes() == b.to_bytes()
    }
}

const SECRET_KEY_BYTES: [u8; 32] = [
    157, 097, 177, 157, 239, 253, 090, 096,
    186, 132, 074, 244, 146, 236, 044, 196,
    068, 073, 197, 105, 123, 050, 105, 025,
    112, 059, 172, 003, 028, 174, 127, 096,
];

#[test]
fn key_secret_key_bytes_have_not_changed() {
    insta::assert_debug_snapshot!(SECRET_KEY_BYTES);
}

#[test]
fn key_snapshot_1() {
    let main = SecretKey::from_bytes(&SECRET_KEY_BYTES).unwrap();
    let keypair = derive_keypair(&main, "hello");
    insta::assert_debug_snapshot!(keypair.to_bytes());
}

#[test]
fn key_snapshot_2() {
    let main = SecretKey::from_bytes(&SECRET_KEY_BYTES).unwrap();
    let keypair = derive_keypair(&main, "hello2");
    insta::assert_debug_snapshot!(keypair.to_bytes());
}

#[test]
fn key_snapshot_3() {
    let main = SecretKey::from_bytes(&SECRET_KEY_BYTES).unwrap();
    let keypair = derive_keypair(
        &main,
        "a very long string as a key name should not break the key derive, \
        it should just work without any issues, this is just testing it, \
        to be sure"
    );
    insta::assert_debug_snapshot!(keypair.to_bytes());
}
