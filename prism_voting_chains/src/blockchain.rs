use crate::block::{self, *};
use crate::crypto::hash::{H256,Hashable};
use log::debug;
use log::info;
use std::collections::HashMap;
use std::collections::VecDeque;
use crate::mempool::{TransactionMempool};
use std::sync::{Arc, Mutex};

// use crate::utils::{*};

extern crate chrono;
use chrono::prelude::*;
use std::cmp;


// Implement remove by element from a Vec
pub fn remove_by_element<T>(list: &mut Vec<T>, element: T) where T: PartialEq {
    let result = list.iter().position(|x| *x == element);
    match result {
        Some(index) => {
            list.remove(index);
        }
        None => {
            // println!("element not present in the list");
        }
    }
}

pub enum InsertStatus {
    Orphan,
    Valid,
}

#[derive(Debug,Clone)]
pub struct Metablock {
    pub block: Block,
    pub level: u32,
}

pub struct Blockchain {
    pub proposer_chain: HashMap<H256, Metablock>,
    pub proposer_tip: H256,
    pub proposer_depth: u32,

    pub voter_chains: Vec<HashMap<H256, Metablock>>,
    pub voter_tips: Vec<H256>,
    pub voter_depths: Vec<u32>,

    // M: list of unreferenced proposer blocks
    pub unref_proposers: Vec<H256>,
    // M: Hash of first proposer block seen corresponding to each level
    pub level2proposer: HashMap<u32, H256>,
    // LM: level -> proposer hash mapping
    pub level2allproposers: HashMap<u32, Vec<H256>>,
    // LM: store the number of votes for each proposer
    pub proposer2votecount: HashMap<H256, u32>,

    //This data structure gives info about the voter blocks who voted for a proposer
    //Basically it gives info about each voter chain and voter block hash which voted for 
    //that proposer
    pub proposer2voterinfo: HashMap<H256, Vec<(u32, H256)>>,

    // Last voted level corresponding to each voter chain
    // IMP TODO: need changes to handle forking in the voter chain
    pub num_voter_chains: u32,
    pub chain2level: HashMap<u32, u32>,

    // orphan buffer stores a mapping between missing reference and block
    // use Vec<Block> as many blocks could wait on a single reference.
    pub orphan_buffer: HashMap<H256, Vec<Block>>,

    // This is the store of all blocks ever received / mined.
    pub blocksdb: HashMap<H256, Block>,

    // reference to mempool
    mempool:Arc<Mutex<TransactionMempool>>, 

    new_proposer: bool,
}

impl Blockchain {
    pub fn new(num_voter_chains: u32, mempool: &Arc<Mutex<TransactionMempool>>) -> Self {
        // genesis for proposer and voter chains
        let mut blocksdb = HashMap::new();

        let mut proposer_chain = HashMap::new();
        let proposer = genesis_proposer();
        let proposer_hash = proposer.hash();
        blocksdb.insert(proposer_hash, proposer.clone());

        let metablock = Metablock {
            block: proposer,
            level: 1,
        };
        proposer_chain.insert(proposer_hash, metablock);
        let proposer_tip = proposer_hash;

        let mut voter_chains = Vec::new();
        let mut voter_tips = Vec::new();
        let mut voter_depths = Vec::new();
        let mut chain2level = HashMap::new();
        for chain_num in 1..(num_voter_chains + 1) {
            let mut tmp_chain = HashMap::new();
            let voter = genesis_voter(chain_num);
            let voter_hash = voter.hash();
            blocksdb.insert(voter_hash, voter.clone());

            let metablock = Metablock {
                block: voter,
                level: 1,
            };
            tmp_chain.insert(voter_hash, metablock);
            voter_chains.push(tmp_chain);
            voter_tips.push(voter_hash);
            voter_depths.push(1);

            chain2level.insert(chain_num, 0);
        } 

        let mut unref_proposers = Vec::new();
        unref_proposers.push(proposer_hash);

        let mut level2proposer = HashMap::new();
        level2proposer.insert(1, proposer_hash);

        let mut level2allproposers = HashMap::new();
        level2allproposers.insert(1, vec![proposer_hash]);

        let mut proposer2votecount = HashMap::new();
        proposer2votecount.insert(proposer_hash, 0);

        let mut proposer2voterinfo = HashMap::new();
        proposer2voterinfo.insert(proposer_hash, Vec::new());

        Blockchain {
            proposer_chain: proposer_chain,
            proposer_tip: proposer_hash,
            proposer_depth: 1,

            voter_chains: voter_chains,
            voter_tips: voter_tips,
            voter_depths: voter_depths,

            unref_proposers: unref_proposers,
            level2proposer: level2proposer,
            level2allproposers: level2allproposers,

            proposer2votecount: proposer2votecount,
            proposer2voterinfo: proposer2voterinfo,
            num_voter_chains: num_voter_chains,
            chain2level: chain2level,

            orphan_buffer: HashMap::new(),
            blocksdb: blocksdb,

            mempool: Arc::clone(mempool),
            new_proposer: true,
        }
    }

    pub fn is_orphan (&mut self, block: &Block) -> bool {
        // If there are missing references, it will add 
        // (first missing ref -> block) entry to orphan buffer map
        match &block.content {
            Content::Proposer(content) => {
                if (!self.proposer_chain.contains_key(&content.parent_hash)) {
                    // parent proposer not found, add to orphan buffer
                    self.orphan_buffer.entry(content.parent_hash).or_insert(Vec::new()).push(block.clone());
                    println!("Adding proposer block with hash {:?} to buffer", block.hash());
                    return true;
                }

                for ref_proposer in content.proposer_refs.clone() {
                    if (!self.proposer_chain.contains_key(&ref_proposer)) {
                        self.orphan_buffer.entry(ref_proposer).or_insert(Vec::new()).push(block.clone());
                        println!("Adding proposer block with hash {:?} to buffer", block.hash());
                        return true;
                    }
                }
                return false;
            }
            Content::Voter(content) => {
                let chain_num = content.chain_num;

                if (!self.voter_chains[(chain_num-1) as usize].contains_key(&content.parent_hash)) {
                    // parent proposer not found, add to orphan buffer
                    self.orphan_buffer.entry(content.parent_hash).or_insert(Vec::new()).push(block.clone());
                    println!("Adding voter block with hash {:?} to buffer", block.hash());
                    return true;
                }

                for vote in content.votes.clone() {
                    if (!self.proposer_chain.contains_key(&vote)) {
                        self.orphan_buffer.entry(vote).or_insert(Vec::new()).push(block.clone());
                        println!("Adding voter block with hash {:?} to buffer", block.hash());
                        return true;
                    }
                }
                return false;
            }
        }
    }

    pub fn insert(&mut self, block: &Block) -> InsertStatus {
        let block_hash = block.hash();
        self.blocksdb.insert(block_hash, block.clone());

        if self.is_orphan(block) {
            return InsertStatus::Orphan;
        }

        // All references inside the block are guaranteed to be present
        match &block.content {
            Content::Proposer(content) => {
                // Remove parent and referenced proposer hashes from `unref_proposers`
                let parent_hash = content.parent_hash;
                remove_by_element(&mut self.unref_proposers, parent_hash);
                // TRY &content.proposer_refs
                for ref_proposer in content.proposer_refs.clone() {
                    remove_by_element(&mut self.unref_proposers, ref_proposer);
                }

                // Add to `proposer_chain` and update tip
                let parent_meta = &self.proposer_chain[&content.parent_hash];
                let block_level = parent_meta.level + 1;
                let metablock = Metablock {
                    // TRY block
                    block: block.clone(),
                    level: block_level,
                };
                self.proposer_chain.insert(block_hash, metablock.clone());
                self.new_proposer = true;
                println!("Added proposer {:?} at level {}", block_hash, block_level);

                if metablock.level > self.proposer_depth {
                    self.proposer_depth = metablock.level;
                    self.proposer_tip = block_hash;
                }

                // remove transactions from the mempool
                // println!("trying to acquire mempool lock");
                let mut locked_mempool = self.mempool.lock().unwrap();
                // println!("acquired mempool lock");

                for tx in &content.transactions {
                    locked_mempool.delete(&tx.hash());
                }
                drop(locked_mempool);

                // Add selfhash to unref_proposers
                self.unref_proposers.push(block_hash);

                // Add to `level2proposer` if first proposer at its level
                if !self.level2proposer.contains_key(&block_level) {
                    self.level2proposer.insert(block_level, block_hash);
                }
                // Add to `level2allproposers`
                self.level2allproposers.entry(block_level).or_insert(Vec::new()).push(block_hash);
            }

            Content::Voter(content) => {
                let chain_num = content.chain_num;

                // BEHOLD
                // The below code is inaccurate: votes aren't counted from every block, 
                // only the blocks belonging to the longest chain. So this is a major TODO.
                // Bhavana will work on this 4/25. 
                
                // go through all votes, update proposer2votecount and chain2level
                let mut max_vote_level: u32 = self.chain2level[&chain_num];
                let voter_info = (chain_num, block_hash);
                for vote in content.votes.clone() {
                    // update proposer2votecount
                    let counter = self.proposer2votecount.entry(vote).or_insert(0);
                    *counter += 1;
                    
                    let voters_info = self.proposer2voterinfo.entry(vote).or_insert(Vec::new());
                    voters_info.push(voter_info);

                    // update max vote level variable
                    let block_level = self.proposer_chain[&vote].level;
                    let max_vote_level = cmp::max(max_vote_level, block_level);
                }
                self.chain2level.insert(chain_num, max_vote_level);

                // add to voter chain and update tip
                let mut parent_meta = &self.voter_chains[(chain_num-1) as usize][&content.parent_hash];
                let metablock = Metablock {
                    block: block.clone(),
                    level: parent_meta.level + 1
                };
                self.voter_chains[(chain_num-1) as usize].insert(block_hash, metablock.clone());
                // println!("Added voter {:?} #{} at level {}", block_hash, chain_num, metablock.level);
                if metablock.level > self.voter_depths[(chain_num-1) as usize] {
                    self.voter_depths[(chain_num-1) as usize] = metablock.level;
                    self.voter_tips[(chain_num-1) as usize] = block_hash;
                }
            }
        }

        let result = self.orphan_buffer.remove(&block_hash);
        match result {
            Some(orphan_blocks) => {
                for orphan_block in &orphan_blocks {
                    let status = self.insert(&orphan_block);
                    match status {
                        InsertStatus::Valid => {
                            // remove orphan_block from the orphan buffer list
                            // let mut orphan_idx: usize = 0;
                            // let mut success: bool = false;
                            // for (list_idx, list_block) in self.orphan_buffer[&block_hash].iter().enumerate() {
                            //     if list_block.hash() == orphan_block.hash() {
                            //         orphan_idx = list_idx;
                            //         success = true;
                            //         break;
                            //     }
                            // }
                            // if success {
                                // self.orphan_buffer[&block_hash].remove(orphan_idx);
                            println!("Orphan block {:?} processed", orphan_block.hash());
                            // }
                        }
                        InsertStatus::Orphan => {},
                    }
                }
            },
            None => {
                // println!("No orphan blocks waiting on {:?}", block_hash)
            },
        }

        InsertStatus::Valid
    }

    pub fn get_proposer_tip(&self) -> H256 {
        self.proposer_tip
    }

    pub fn get_voter_tip(&self, chain_num: u32) -> H256 {
        self.voter_tips[(chain_num-1) as usize]
    }

    pub fn get_unref_proposers(&self) -> Vec<H256> {
        self.unref_proposers.clone()
    }

    pub fn get_votes(&self, chain_num: u32) -> Vec<H256> {
        let mut votes: Vec<H256> = Vec::new();
        let last_voted_level = self.chain2level[&chain_num];
        let last_proposer_level = self.proposer_chain[&self.proposer_tip].level;
        for level in (last_voted_level+1)..(last_proposer_level+1) {
            votes.push(self.level2proposer[&level]);
        }
        votes
    }

    pub fn has_block(&self, block_hash: H256) -> bool {
        self.blocksdb.contains_key(&block_hash)
    }

    pub fn get_block(&self, block_hash: H256) -> Option<&Block> {
        self.blocksdb.get(&block_hash)
    }

    pub fn has_new_proposer(&mut self) -> bool {
        let ret_val = self.new_proposer;
        self.new_proposer = false;
        ret_val
    }

    pub fn print_chains(&self) {
        let mut chain: Vec<Vec<H256>> = Vec::new();

        for level in 1..(self.proposer_depth+1) {
            chain.push(self.level2allproposers[&level].clone());
        }

        println!("proposers: {:?}", chain);

        for chain_num in 1..(self.num_voter_chains+1) {
            let chain_idx = chain_num - 1;
            let mut voter_chain: Vec<H256> = Vec::new();
            let mut curr_key = self.voter_tips[chain_idx as usize];
            while (self.voter_chains[chain_idx as usize].contains_key(&curr_key)) {
                voter_chain.push(curr_key);
                let content = &self.voter_chains[chain_idx as usize].get(&curr_key).unwrap().block.content;
                match (content) {
                    Content::Voter(c) => {
                        curr_key = c.parent_hash;
                    }
                    Content::Proposer(c) => {
                        println!("wtf is wrong with you? you're a voter");
                    }
                }
            }
            voter_chain.reverse();
            println!("voter chain {}: {:?}", chain_num, voter_chain);
        }
    }

}

// write tests for blockchain
#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    // use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn blockchain_init() {
        // 10 voting chains
        let mut blockchain = Blockchain::new(10);
    }
}
