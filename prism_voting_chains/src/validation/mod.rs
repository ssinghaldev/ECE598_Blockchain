
use crate::network::server::Handle as ServerHandle;
use crate::block::{self, *};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::merkle::{MerkleTree, verify};
use crate::blockchain::{Blockchain, InsertStatus};
use crate::miner::{sortition_hash, PROPOSER_INDEX, FIRST_VOTER_IDX};

use log::info;
use bigint::uint::U256;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

pub enum BlockResult {
    Pass,
    Fail,
}

//PoW and sortition id
pub fn check_pow_sortition_id(block: &Block, num_voter_chains: u32) -> BlockResult {
    let sortition_id = sortition_hash(block.hash(), block.header.difficulty, num_voter_chains);
    if sortition_id.is_none() {
        println!("New block does not satisy proof-of-work");
        return BlockResult::Fail;
    }

    let correct_sortition_id = match &block.content {
        Content::Proposer(_) => PROPOSER_INDEX,
        Content::Voter(content) => content.chain_num,
    };
    if sortition_id.unwrap() != correct_sortition_id {
        println!("Sortition check failed: sortition hash {} content mapping {}", sortition_id.unwrap(), correct_sortition_id);
        return BlockResult::Fail;
    }
    return BlockResult::Pass;
}

//check merkle tree there
pub fn check_sortition_proof(block: &Block, num_voter_chains: u32) -> BlockResult {
    let sortition_id = sortition_hash(block.hash(), block.header.difficulty, num_voter_chains);
    if sortition_id.is_none() {
        return BlockResult::Fail;
    }
    if !verify(
        &block.header.merkle_root,
        &block.content.hash(),
        &block.sortition_proof,
        sortition_id.unwrap() as usize,
        (num_voter_chains + FIRST_VOTER_IDX) as usize,
    ) {
        return BlockResult::Fail;
    }
    return BlockResult::Pass;
}
