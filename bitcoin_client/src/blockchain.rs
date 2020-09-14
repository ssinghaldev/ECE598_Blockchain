use crate::block::{self, *};
use crate::crypto::hash::{H256,Hashable};
use log::info;
use std::collections::HashMap;
use std::collections::VecDeque;

extern crate chrono;
use chrono::prelude::*;

pub struct Blockchain {
    pub chain:HashMap<H256,Block>,
    pub tiphash:H256,
    pub heights:HashMap<H256,u8>,
    pub buffer:HashMap<H256,Block>,
    pub totaldelay:i64,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let buffer: [u8; 32] = [0; 32];
        let b:H256 = buffer.into();
        let genesis:Block = block::generate_genesis_block(&b);
        let genhash:H256 = genesis.hash();
        let mut chainmap:HashMap<H256,Block> = HashMap::new();
        let mut heightsmap:HashMap<H256,u8> = HashMap::new();
        let buffermap:HashMap<H256,Block> = HashMap::new();
        chainmap.insert(genhash,genesis);
        heightsmap.insert(genhash,0);
        let t:H256 = genhash;
        let newchain:Blockchain = Blockchain{chain:chainmap,tiphash:t,heights:heightsmap,buffer:buffermap,totaldelay:0};
        newchain
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {


        let h:H256 = block.hash();
        //let mut flag:bool = false;


        match self.chain.get(&block.header.parenthash){
            Some(pblock) => { //insertion into mainchain

                if h < pblock.header.difficulty && !self.chain.contains_key(&h) {
                let b_delay = Local::now().timestamp_millis() - block.header.timestamp;
                self.totaldelay = self.totaldelay + b_delay;
                info!("Adding block with hash {} to chain",h);
                println!("Block delay is: {:?}",(Local::now().timestamp_millis() - block.header.timestamp));
                println!("Average delay is {}",self.totaldelay/(self.chain.len() as i64));
                println!("Total number of blocks in blockchain:{}\n",self.chain.len());
                self.chain.insert(h,block.clone());
                let len = self.heights[&block.header.parenthash]+1;
                self.heights.insert(h,len);
                if len>self.heights[&self.tiphash] {
                    self.tiphash = h;
                }

                //let mut bhash_copy:H256 = hash::generate_random_hash();
                //if stale blocks parent has arrived, insert it into main chain
                let mut bhash_vec = Vec::new();
                let mut phash_q: VecDeque<H256>= VecDeque::new();
                phash_q.push_back(h);
                while !phash_q.is_empty() {
                    match phash_q.pop_front() {
                        Some(h) => for (bhash,blck) in self.buffer.iter(){
                                if blck.header.parenthash == h {
                                    //flag = true;
                                    let bhash_copy:H256 = *bhash;
                                    bhash_vec.push(bhash_copy);
                                    self.chain.insert(bhash_copy,blck.clone());
                                    let b_delay = Local::now().timestamp_millis() - block.header.timestamp;
                                    self.totaldelay = self.totaldelay + b_delay;
                                    info!("Adding block with hash {} to chain",blck.hash());
                                    println!("Block delay is: {:?}",(Local::now().timestamp_millis() - blck.header.timestamp));
                                    println!("Average delay is {}",self.totaldelay/(self.chain.len() as i64));
                                    println!("Total number of blocks in blockchain:{}\n",self.chain.len());
                                    let len = self.heights[&blck.header.parenthash]+1;
                                    self.heights.insert(bhash_copy,len);
                                    if len>self.heights[&self.tiphash] {
                                        self.tiphash = bhash_copy;
                                    }
                                }
                            },
                        None => (),
                    }
                }


                for bh in bhash_vec{
                    self.buffer.remove(&bh);
                }
             }
            }, // insert stale block into buffer
            _ => {
                  print!("Adding block with hash {} to buffer\n",h); 
                  if !self.buffer.contains_key(&h){
                  self.buffer.insert(h,block.clone()); 
                  }
                 },
        }

    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tiphash
    }

    /// Get the last block's hash of the longest chain
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {

        let mut phash:H256 = self.tiphash;
        let mut result:Vec<H256>=vec![];
        let mut buffer: [u8; 32] = [0; 32];
        let b:H256 = buffer.into();
        while(phash!=b){
            result.push(phash);
            phash = self.chain[&phash].header.parenthash;
        }
        let mut res = result.reverse();
        result
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = block::generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }
}
