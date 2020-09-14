//#[cfg(test)]
//#[macro_use]

use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use crate::transaction::{self, SignedTransaction};
use crate::crypto::merkle::MerkleTree;

extern crate chrono;
use chrono::prelude::*;

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Header {
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root:H256,
    pub miner_id:i32,
}
#[derive(Serialize, Deserialize, Debug,Clone)]
// content depends on type of block, represented by enum
pub enum Content {
    Proposer(ProposerContent),
    Voter(VoterContent),
}

impl Hashable for Content {
    fn hash(&self) -> H256 {
        match self {
            Content::Proposer(c) => c.hash(),
            Content::Voter(c) => c.hash(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Block {
    pub header:Header,
    pub content:Content,
    pub sortition_proof:Vec<H256>,
}


impl Block {
    pub fn new(
        //parent: H256,
        ts: u128,
        n: u32,
        content_merkle_root: H256,
        sortition_proof: Vec<H256>,
        content: Content,
        miner_identity:i32,
        diff: H256,
    ) -> Self {
        let header = Header{
           // parenthash:parent,
            nonce:n,
            difficulty:diff,
            timestamp:ts,
            merkle_root:content_merkle_root,
            miner_id:miner_identity,
        };
        Self {
            header,
            content,
            sortition_proof,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ProposerContent {
    pub parent_hash:H256,
    pub transactions: Vec<SignedTransaction>,
    pub proposer_refs: Vec<H256>,
}

impl Hashable for ProposerContent {
    fn hash(&self) -> H256 {
        let txns_merkle_tree = MerkleTree::new(&self.transactions);
        let prop_refs_merkle_tree = MerkleTree::new(&self.proposer_refs);
        let mut byte_array = [0u8; 96];
        byte_array[..32].copy_from_slice(self.parent_hash.as_ref());
        byte_array[32..64].copy_from_slice(prop_refs_merkle_tree.root().as_ref());
        byte_array[64..96].copy_from_slice(txns_merkle_tree.root().as_ref());
        ring::digest::digest(&ring::digest::SHA256, &byte_array).into()
    }
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct VoterContent {
    pub votes: Vec<H256>, // hashes of blocks to which votes have been cast
    pub parent_hash : H256, //hash of parent block
    pub chain_num: u32, //chain number of voter block 
}

impl Hashable for  VoterContent {
    fn hash(&self)->H256 {
        let merkle_tree =  MerkleTree::new(&self.votes);
        let mut bytes = [0u8; 68];
        bytes[..4].copy_from_slice(&self.chain_num.to_be_bytes());
        bytes[4..36].copy_from_slice(self.parent_hash.as_ref());
        bytes[36..68].copy_from_slice(merkle_tree.root().as_ref());
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
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

/*pub fn generate_random_block(parent: &H256) -> Block {
    let mut rng = rand::thread_rng();
    let r1:u32 = rng.gen();
    //let r2:u128 = rng.gen();
    let local: DateTime<Local> = Local::now();

    //let mut buffer: [u8; 32] = [0; 32];
    let b:H256 = hex!("99911718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into();
    let h:Header = Header{parenthash:*parent,nonce:r1,difficulty:b,timestamp:local.timestamp_millis(),merkle_root:b,miner_id:-1};
    let t = transaction::generate_random_signed_transaction();
    //transaction::pr();
    let mut vect:Vec<SignedTransaction> = vec![];
    vect.push(t);
    let c:Content = Content{data:vect};
    let b:Block = Block{header:h,content:c};
    b
}

pub fn generate_genesis_block(parent: &H256) -> Block {
    //let b:H256 = hex!("00011718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into();
    let b:H256 = hex!("99911718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into();
    let r1:u32 = 0;
    let r2:i64 = 0;
    //let local: DateTime<Local> = Local::now();
    let h:Header = Header{parenthash:*parent,nonce:r1,difficulty:b,timestamp:r2,merkle_root:b,miner_id:-1};
    let t = transaction::generate_genesis_signed_transaction();
    //transaction::pr();
    let mut vect:Vec<SignedTransaction> = vec![];
    vect.push(t);
    let c:Content = Content{data:vect};
    let b:Block = Block{header:h,content:c};
    b
}*/

pub fn genesis_proposer() -> Block {
let zero_vec : [u8; 32] = [0; 32];
   let content = ProposerContent {
      parent_hash:zero_vec.into(),
      transactions:vec![],
      proposer_refs:vec![],
   };
   
   let raw: [u8; 32] = [255; 32];
   let default_diff:H256= raw.into();
   Block::new(0,0,zero_vec.into(),vec![],Content::Proposer(content),0,default_diff,)
}

pub fn genesis_voter(chain_number:u32) -> Block {
    let zero_vec : [u8; 32] = [0; 32];
    let content = VoterContent {
        chain_num: chain_number,
        parent_hash: zero_vec.into(),
        votes: vec![],
    };
    let raw: [u8; 32] = [255; 32];
    let default_diff:H256= raw.into();

    Block::new(0,0,zero_vec.into(),vec![],Content::Voter(content),0,default_diff,)
}

