extern crate bincode;
extern crate serde;

use serde::{Serialize,Deserialize};
use ring::signature::{self,Ed25519KeyPair, Signature, KeyPair};

use crate::crypto::key_pair;
use crate::crypto::hash::{self, H256, Hashable};
use crate::crypto::address::{self, H160};

#[derive(Serialize, Deserialize, Debug, Default,Clone, Eq, PartialEq, Hash)]
pub struct UtxoInput{
  pub tx_hash: H256,
  pub idx: u8,  
}

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct UtxoOutput{
  pub receipient_addr: H160,
  pub value: u32,
}

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct Transaction {
  pub tx_input: Vec<UtxoInput>,
  pub tx_output: Vec<UtxoOutput>,
}

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct SignedTransaction {
  pub tx: Transaction,
  pub signature: Vec<u8>, 
  pub public_key: Vec<u8>,
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let encodedtrans: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encodedtrans[..]).into()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let encoded_signed_trans: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encoded_signed_trans[..]).into()
    }
}


/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    //let mut mess = [&t.input[..],&t.output[..]].concat();
    let encoded: Vec<u8> = bincode::serialize(&t).unwrap();
    //let merged : Vec<_> =mess.iter().flat_map(|s.as_mut()| s.iter()).collect();
    let sig = key.sign(&encoded[..]);
    sig
}

/*
/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    //let mut mess = [&t.input[..],&t.output[..]].concat();
    let encoded: Vec<u8> = bincode::serialize(&t).unwrap();
    let public_key_bytes = public_key.as_ref();
    let peer_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key_bytes);
    peer_public_key.verify(&encoded[..],signature.as_ref()).is_ok()
}*/

pub fn verify(t: &Transaction, signature_bytes: &Vec<u8>, public_key_bytes: &Vec<u8>) -> bool {
    //let mut mess = [&t.input[..],&t.output[..]].concat();
    let encoded: Vec<u8> = bincode::serialize(&t).unwrap();
    let peer_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key_bytes);
    peer_public_key.verify(&encoded[..],signature_bytes).is_ok()
}

pub fn generate_random_transaction() -> Transaction {
    let input = vec![UtxoInput{tx_hash: hash::generate_random_hash(), idx: 0}];
    let output = vec![UtxoOutput{receipient_addr: address::generate_random_address(), value: 0}];
    
    Transaction{tx_input: input, tx_output: output}
}

pub fn generate_genesis_transaction() -> Transaction {
    let input = vec![UtxoInput{tx_hash: H256::from([0;32]), idx: 0}];
    let output = vec![UtxoOutput{receipient_addr: H160::from([0;20]), value: 0}];
    
    Transaction{tx_input: input, tx_output: output}
}

pub fn generate_random_signed_transaction() -> SignedTransaction {
    let t = generate_random_transaction();
    let key = key_pair::random();
    let sig = sign(&t, &key);
    let signed_tx = SignedTransaction{tx:t,
                                      signature:sig.as_ref().to_vec(),
                                      public_key:key.public_key().as_ref().to_vec()};
    signed_tx
}

/*
pub fn generate_genesis_signed_transaction() -> SignedTransaction {
    let t = generate_genesis_transaction();
    let key = key_pair::random();
    let sig = sign(&t, &key);
    let signed_tx = SignedTransaction{tx:t,
                                      signature:sig.as_ref().to_vec(),
                                      public_key:key.public_key().as_ref().to_vec()};
    signed_tx
}*/



pub fn generate_genesis_signed_transaction() -> SignedTransaction {

    let t = generate_genesis_transaction();
    let key = Ed25519KeyPair::from_pkcs8([48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 187, 131, 74, 161, 134, 11, 240, 6, 188, 109, 18, 108, 124, 219, 167, 164, 215, 125, 168, 79, 204, 194, 232, 91, 58, 186, 181, 230, 212, 78, 163, 28, 161, 35, 3, 33, 0, 233, 72, 146, 218, 220, 235, 17, 123, 202, 112, 119, 63, 134, 105, 134, 71, 34, 185, 71, 193, 59, 66, 43, 137, 50, 194, 120, 234, 97, 132, 235, 159].as_ref().into()).unwrap();
    let sig = sign(&t, &key);
    let signed_tx = SignedTransaction{tx:t,signature:sig.as_ref().to_vec(),public_key:key.public_key().as_ref().to_vec()};
    signed_tx
}

#[cfg(any(test, test_utilities))]
pub mod tests {
    use super::*;
    use crate::crypto::key_pair;
   
    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &signature.as_ref().to_vec(), &key.public_key().as_ref().to_vec()));
    }
}
