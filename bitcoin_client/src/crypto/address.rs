use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, KeyPair};
use ring::digest;
use rand::Rng;

//Last 20 bytes of Public Key - used in tx
#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct H160([u8; 20]);

impl std::fmt::Debug for H160 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[18], &self.0[19]
        )
    }
}

impl std::convert::AsRef<[u8]> for H160 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::convert::From<&[u8; 20]> for H160 {
    fn from(input: &[u8; 20]) -> H160 {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(input);
        H160(buffer)
    }
}

impl std::convert::From<&H160> for [u8; 20] {
    fn from(input: &H160) -> [u8; 20] {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(&input.0);
        buffer
    }
}

impl std::convert::From<[u8; 20]> for H160 {
    fn from(input: [u8; 20]) -> H160 {
        H160(input)
    }
}

impl std::convert::From<H160> for [u8; 20] {
    fn from(input: H160) -> [u8; 20] {
        input.0
    }
}

pub fn address_from_public_key_ref(public_key: &<Ed25519KeyPair as KeyPair>::PublicKey) -> H160 {
    let public_key_hash = digest::digest(&digest::SHA256, public_key.as_ref());
    
    let mut raw_address: [u8; 20] = [0; 20];
    raw_address.copy_from_slice(&(public_key_hash.as_ref()[12..32]));
    H160(raw_address)
}

pub fn address_from_public_key(public_key: <Ed25519KeyPair as KeyPair>::PublicKey) -> H160 {
    let public_key_hash = digest::digest(&digest::SHA256, public_key.as_ref());

    let mut raw_address: [u8; 20] = [0; 20];
    raw_address.copy_from_slice(&(public_key_hash.as_ref()[12..32]));
    H160(raw_address)
}

pub fn address_from_public_key_vec_ref(public_key: &Vec<u8>) -> H160 {
    let public_key_hash = digest::digest(&digest::SHA256, public_key);

    let mut raw_address: [u8; 20] = [0; 20];
    raw_address.copy_from_slice(&(public_key_hash.as_ref()[12..32]));
    H160(raw_address)
}

pub fn generate_random_address() -> H160 {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    let mut raw_bytes = [0; 20];
    raw_bytes.copy_from_slice(&random_bytes);
    (&raw_bytes).into()
}
