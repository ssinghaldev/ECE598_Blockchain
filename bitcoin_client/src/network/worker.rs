use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::block::*;
use crate::transaction::SignedTransaction;
use crate::transaction_checks;
use crate::mempool::TransactionMempool;
use crate::crypto::hash::{H256, Hashable};

use crossbeam::channel;
use log::{debug, warn};

use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_mempool: Arc<Mutex<TransactionMempool>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_mempool: &Arc<Mutex<TransactionMempool>>
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        tx_mempool: Arc::clone(tx_mempool)
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
            let mut locked_blockchain = self.blockchain.lock().unwrap();
            let mut locked_mempool = self.tx_mempool.lock().unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(vec_hashes) => {
                    let mut required_blocks:Vec<H256> = vec![];
                    debug!("Received New Block Hashes");

                    for recv_hash in vec_hashes {
                        let mut flag: bool = false;
                        for (bhash,_) in locked_blockchain.chain.iter(){
                            if *bhash == recv_hash{
                                debug!("Block that hashes to {} already present", bhash);
                                flag = true;
                            }
                        }
                        for (bhash,_) in locked_blockchain.buffer.iter(){
                            if *bhash == recv_hash {
                                debug!("Block that hashes to {} already present", bhash);
                                flag = true;
                            }
                        }
                        if !flag {
                            required_blocks.push(recv_hash);
                        }
                    }
                    if required_blocks.len()!= 0 {
                        debug!("Sending getBlocks Message");
                        peer.write(Message::GetBlocks(required_blocks));
                    }

                }
                Message::GetBlocks(vec_hashes) => {
                    let mut give_blocks:Vec<Block> = vec![];
                    debug!("Received GetBlocks");
                    for getblock_hash in vec_hashes {
                        for (bhash,blck) in locked_blockchain.chain.iter(){
                            if *bhash == getblock_hash{
                                debug!("Adding block with hash {} to give_blocks", bhash);
                                give_blocks.push(blck.clone());
                            }
                        }
                        for (bhash,blck) in locked_blockchain.buffer.iter(){
                            if *bhash == getblock_hash {
                                debug!("Adding block with hash {} to give_blocks", bhash);
                                give_blocks.push(blck.clone());
                            }
                        }

                    }
                    if give_blocks.len()!=0 {
                        debug!("Sending Blocks message");
                        peer.write(Message::Blocks(give_blocks));
                    }

                }
                Message::Blocks(vec_blocks) => {
                    debug!("Received Blocks message");
                    for blck in vec_blocks {
                      let mut blck_is_valid: bool = true;
                      for tx in &blck.content.data{
                          if !transaction_checks::is_tx_valid(tx){
                             blck_is_valid = false;
                             debug!("Invalid tx in received block. Ignoring that block");
                             break;
                          }
                      }

                      if blck_is_valid {
                        // added difficulty check in insert method
                        locked_blockchain.insert(&blck);
                        
                        //Sending getblocks message if block is orphan
                        let mut get_block_hash : Vec<H256> = vec![];
                        get_block_hash.push(blck.header.parenthash);
                        if !locked_blockchain.chain.contains_key(&blck.header.parenthash){
                            self.server.broadcast(Message::GetBlocks(get_block_hash));
                        }

                        //broadcasting NewBlockHashes
                        let mut new_block_hash : Vec<H256> = vec![];
                        new_block_hash.push(blck.hash());
                        self.server.broadcast(Message::NewBlockHashes(new_block_hash));

                        //Updating mempool
                        for signed_tx in &blck.content.data {
                          let signed_tx_hash = signed_tx.hash();
                          match locked_mempool.tx_to_process.get(&signed_tx_hash){
                              Some(_tx_present) => {
                                  locked_mempool.tx_to_process.insert(signed_tx_hash, false);
                              },
                              None => {
                                  locked_mempool.tx_to_process.insert(signed_tx_hash, false);
                                  locked_mempool.tx_map.insert(signed_tx_hash, signed_tx.clone());
                              }
                          }
                        }

                        //Updating State
                      
                      }
                    }
                }
                Message::NewTransactionHashes(vec_tx_hashes) => {
                    let mut required_txs: Vec<H256> = vec![];
                    debug!("Received NewTransactionHashes");
                    
                    for recv_tx_hash in vec_tx_hashes {
                        match locked_mempool.tx_to_process.get(&recv_tx_hash){
                            Some(_tx_present) => debug!("tx which hashes to {} already present in mempool", 
                                                        recv_tx_hash),
                            None => required_txs.push(recv_tx_hash.clone())
                        }
                    }

                    if required_txs.len()!= 0 {
                        debug!("Sending GetTransactions Message");
                        peer.write(Message::GetTransactions(required_txs));
                    }
                }
                Message::GetTransactions(vec_tx_hashes) => {
                    let mut txs_to_send:Vec<SignedTransaction> = vec![];
                    debug!("Received GetTransactions");
                    
                    for tx_hash in vec_tx_hashes {
                        match locked_mempool.tx_map.get(&tx_hash){
                            Some(signed_tx) => txs_to_send.push(signed_tx.clone()), 
                            None => debug!("tx which hashes to {} not present in mempool", tx_hash)
                        }
                    }
                    
                    if txs_to_send.len()!=0 {
                        debug!("Sending Transactions message");
                        peer.write(Message::Transactions(txs_to_send));
                    }
                }
                Message::Transactions(vec_signed_txs) => {
                    debug!("Received Transactions");
                    let mut tx_hashes_to_broadcast: Vec<H256> = vec![];
                    for signed_tx in vec_signed_txs {
                      if transaction_checks::is_tx_valid(&signed_tx){
                          let signed_tx_hash = signed_tx.hash();
                          match locked_mempool.tx_to_process.get(&signed_tx_hash){
                              Some(_tx_present) => debug!("tx_hash {} already present. Not adding to mempool", 
                                                         signed_tx_hash),
                              None => {
                                  locked_mempool.tx_to_process.insert(signed_tx_hash, true);
                                  locked_mempool.tx_map.insert(signed_tx_hash, signed_tx);
                                  locked_mempool.tx_hash_queue.push_back(signed_tx_hash);
                                  tx_hashes_to_broadcast.push(signed_tx_hash);
                              }
                          }
                      }
                    }
                    if tx_hashes_to_broadcast.len() != 0{
                      self.server.broadcast(Message::NewTransactionHashes(tx_hashes_to_broadcast));
                    }
                }
            }
        }
    }
}
