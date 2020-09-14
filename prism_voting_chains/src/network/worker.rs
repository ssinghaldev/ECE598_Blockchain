// use super::buffer::BlockBuffer;
use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::{Blockchain, InsertStatus};
use crate::block::*;
use crate::transaction::SignedTransaction;
use crate::mempool::TransactionMempool;
use crate::crypto::hash::{H256, Hashable};
use std::collections::{HashMap, HashSet};
// use crate::validation::{BlockResult};
use crossbeam::channel;
use log::{info,debug, warn};
use crate::validation::{BlockResult, check_pow_sortition_id, check_sortition_proof};

use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<TransactionMempool>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<TransactionMempool>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            
            match msg {
                Message::Ping(nonce) => {
                    println!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    println!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(vec_hashes) => {
                    let mut req_blocks = Vec::new();
                    let locked_blockchain = self.blockchain.lock().unwrap();
                    for block_hash in vec_hashes {
                        if (!locked_blockchain.has_block(block_hash)) {
                            req_blocks.push(block_hash);
                        }
                    }
                    drop(locked_blockchain);

                    if req_blocks.len() > 0 {
                        peer.write(Message::GetBlocks(req_blocks));
                    }
                }

                Message::GetBlocks(vec_hashes) => {
                    let mut newblocks: Vec<Block> = Vec::new();
                    let locked_blockchain = self.blockchain.lock().unwrap();
                    for block_hash in vec_hashes {
                        let result = locked_blockchain.get_block(block_hash);
                        match result {
                            Some(block) => {
                                newblocks.push(block.clone());
                            }
                            None => {
                                println!("blocksdb does not contain {}", block_hash);
                            }
                        }
                    }
                    drop(locked_blockchain);

                    if newblocks.len() > 0 {
                        peer.write(Message::Blocks(newblocks));
                    }
                }

                Message::Blocks(vec_blocks) => {
                    let mut locked_blockchain = self.blockchain.lock().unwrap();
                    let num_voter_chains = locked_blockchain.num_voter_chains;
                    let mut valid_block_hashes: Vec<H256> = Vec::new();
                    for block in vec_blocks {
                        let block_hash = block.hash();
                        if (!locked_blockchain.has_block(block_hash)) {
                            // perform validation checks -- hash < difficulty, sortition id, sortition proof
                            let result = check_pow_sortition_id(&block, num_voter_chains);
                            match result {
                                BlockResult::Fail => {
                                    println!("Invalid block {:?} pow/sortition failed", block_hash);
                                    continue;
                                }
                                BlockResult::Pass => {
                                    // println!("pow/sortition passed {:?}", block_hash);
                                    let result2 = check_sortition_proof(&block, num_voter_chains);
                                    match result2 {
                                        BlockResult::Fail => {
                                            println!("Invalid block {:?} sortition proof failed", block_hash);
                                            continue;
                                        }
                                        BlockResult::Pass => {
                                            // println!("both checks passed {:?}", block_hash);
                                        }
                                    }
                                }
                            }
                            let result = locked_blockchain.insert(&block);
                            if let result = InsertStatus::Valid {
                                valid_block_hashes.push(block_hash);
                            }
                        }
                    } 
                    drop(locked_blockchain);
                    if valid_block_hashes.len() > 0 {
                        self.server.broadcast(Message::NewBlockHashes(valid_block_hashes));
                    }
                }

                Message::NewTransactionHashes(vec_tx_hashes) => {
                    let mut req_txs: Vec<H256> = vec![];
                    // println!("Received NewTransactionHashes");
                    let locked_mempool = self.mempool.lock().unwrap();
                    for tx_hash in vec_tx_hashes {
                        if (!locked_mempool.contains(&tx_hash)) {
                            req_txs.push(tx_hash);
                        }
                    }
                    drop(locked_mempool);
                    if req_txs.len() > 0 {
                        peer.write(Message::GetTransactions(req_txs));
                    }
                }

                Message::GetTransactions(vec_tx_hashes) => {
                    let mut newtrxs: Vec<SignedTransaction> = Vec::new();
                    let locked_mempool = self.mempool.lock().unwrap();
                    for tx_hash in vec_tx_hashes {
                        let result = locked_mempool.get(&tx_hash);
                        match result {
                            Some(txstore) => newtrxs.push(txstore.signed_tx.clone()),
                            None => println!("mempool does not contain {}", tx_hash),
                        }
                    }
                    drop(locked_mempool);
                    if newtrxs.len() > 0 {
                        // println!("Sending Transactions message");
                        peer.write(Message::Transactions(newtrxs));
                    }
                }

                Message::Transactions(vec_txs) => {
                    let mut locked_mempool = self.mempool.lock().unwrap();
                    let mut new_tx_hashes: Vec<H256> = Vec::new();
                    for tx in vec_txs {
                        let tx_hash = tx.hash();
                        if (!locked_mempool.contains(&tx_hash)) {
                            locked_mempool.insert(tx);
                            new_tx_hashes.push(tx_hash);
                        }
                    }
                    drop(locked_mempool);
                    if new_tx_hashes.len() > 0{
                        self.server.broadcast(Message::NewTransactionHashes(new_tx_hashes));
                    }
                }

            }
        }
    }
}
