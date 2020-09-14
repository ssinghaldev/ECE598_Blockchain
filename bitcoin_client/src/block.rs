//#[cfg(test)]
//#[macro_use]

use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use crate::transaction::{self, SignedTransaction};

extern crate chrono;
use chrono::prelude::*;

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Header {
    pub parenthash: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: i64,
    pub merkle_root:H256,
}
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Content {
    pub data:Vec<SignedTransaction>,
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Block {
    pub header:Header,
    pub content:Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let encodedhead: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encodedhead[..]).into()
    }
}

pub fn generate_random_block(parent: &H256) -> Block {
    let mut rng = rand::thread_rng();
    let r1:u32 = rng.gen();
    //let r2:u128 = rng.gen();
    let local: DateTime<Local> = Local::now();

    //let mut buffer: [u8; 32] = [0; 32];
    let b:H256 = hex!("00011718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into();
    let h:Header = Header{parenthash:*parent,nonce:r1,difficulty:b,timestamp:local.timestamp_millis(),merkle_root:b};
    let t = transaction::generate_random_signed_transaction();
    //transaction::pr();
    let mut vect:Vec<SignedTransaction> = vec![];
    vect.push(t);
    let c:Content = Content{data:vect};
    let b:Block = Block{header:h,content:c};
    b
}

pub fn generate_genesis_block(parent: &H256) -> Block {
    let b:H256 = hex!("00011718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into();
    let r1:u32 = 0;
    let r2:i64 = 0;
    //let local: DateTime<Local> = Local::now();
    let h:Header = Header{parenthash:*parent,nonce:r1,difficulty:b,timestamp:r2,merkle_root:b};
    let t = transaction::generate_genesis_signed_transaction();
    //transaction::pr();
    let mut vect:Vec<SignedTransaction> = vec![];
    vect.push(t);
    let c:Content = Content{data:vect};
    let b:Block = Block{header:h,content:c};
    b
}
