use crate::network::server::Handle as ServerHandle;
use crate::block::{self, *};
use crate::blockchain::{Blockchain};
use crate::crypto::hash::{H256, Hashable};
use crate::mempool::{TransactionMempool};
use crate::crypto::merkle::MerkleTree;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use crate::network::message::{Message};
use log::info;
use bigint::uint::U256;
use rand::Rng;
use crate::transaction::{self, SignedTransaction};

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

const TOTAL_SORTITION_WIDTH: u64 = std::u64::MAX;
pub const PROPOSER_INDEX: u32 = 0;
pub const FIRST_VOTER_IDX: u32 = 1;

pub struct Superblock {
    pub header: Header,
    pub content: Vec<Content>,
}

impl Hashable for Superblock {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

pub fn get_difficulty(num_voter_chains: u32) -> H256 {
    let base_difficulty: H256 = (hex!("0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")).into();
    let difficulty = U256::from_big_endian(base_difficulty.as_ref());
    let adjusted_difficulty = difficulty * (num_voter_chains + 1).into();
    let mut buffer: [u8; 32] = [0; 32];
    adjusted_difficulty.to_big_endian(&mut buffer);
    buffer.into()    
}


pub fn sortition_hash(hash: H256, difficulty: H256, num_voter_chains: u32) -> Option<u32> {
    let hash = U256::from_big_endian(hash.as_ref());
    let difficulty = U256::from_big_endian(difficulty.as_ref());
    let multiplier = difficulty / TOTAL_SORTITION_WIDTH.into();
    
    let precise: f32 = (1.0 / (num_voter_chains + 1) as f32) * TOTAL_SORTITION_WIDTH as f32;
    let proposer_sortition_width: u64 = precise.ceil() as u64;
    let proposer_width = multiplier * proposer_sortition_width.into();
    if hash < proposer_width {
        Some(PROPOSER_INDEX)
    } else if hash < difficulty {
        let voter_idx = (hash - proposer_width) % num_voter_chains.into();
        Some(FIRST_VOTER_IDX + voter_idx.as_u32())
    } else {
        println!("Why you sortitioning something that is not less than difficulty?");
        None
    }
}

enum ControlSignal {
    Start(u64,u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64,u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool:Arc<Mutex<TransactionMempool>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<TransactionMempool>>,  
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool)
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64,index: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda,index))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i,j) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i,j);
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        // let experiment_duration = 300;   // in secs
        // let start = Instant::now();
        // let mut num_proposer_blocks = 0;

        loop {
            let mut index:u64 = 0;
            let mut time_i:u64 = 0;

            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {},
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }

            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            if let OperatingState::Run(i, j) = self.operating_state {
                index = j;
                time_i = i; 
                
                if time_i != 0 {
                    let interval = time::Duration::from_micros(time_i as u64);
                    thread::sleep(interval);
                }

                let mut txs: Vec<SignedTransaction> = Vec::new();

                let locked_mempool = self.mempool.lock().unwrap();
                // println!("miner: acquired mempool lock");
                if (locked_mempool.len() == 0) {
                    drop(locked_mempool);
                    // println!("miner: dropped mempool lock");
                    // println!("Mempool is empty, see ya later, sleeping");
                    let interval = time::Duration::from_micros(time_i as u64);
                    thread::sleep(interval);
                } else {
                    txs = locked_mempool.get_transactions(5);
                    // println!("length of txs in miner {}", txs.len());
                    drop(locked_mempool);
                    // println!("miner: dropped mempool lock");
                }

                let mut empty_mempool:bool = false; 
                
                while (true) {
                    // step1: assemble a new superblock
                    // TODO: We can optimize the assembly by using the version numbers trick
                    // println!("miner: acquired blockchain lock");
                    let mut locked_blockchain = self.blockchain.lock().unwrap();
                    // println!("miner: dropped blockchain lock");

                    // if locked_blockchain.proposer_chain.len() == 10 {
                    //     info!("[Miner] finished experiment, shutting down ...");
                    //     self.operating_state = OperatingState::ShutDown;
                    //     break;
                    // }
    
                    if (locked_blockchain.has_new_proposer() || empty_mempool) {
                        let locked_mempool = self.mempool.lock().unwrap();
                        // println!("miner: acquired mempool lock");
                        if (locked_mempool.len() == 0) {
                            drop(locked_mempool);
                            // println!("miner: dropped mempool lock");
                            // println!("Mempool is empty, see ya later, sleeping");
                            let interval = time::Duration::from_micros(time_i as u64);
                            thread::sleep(interval);
                        } else {
                            txs = locked_mempool.get_transactions(5);
                            // println!("length of txs in miner {}", txs.len());
                            drop(locked_mempool);
                            // println!("miner: dropped mempool lock");
                        }
                    }

                    if (txs.len() == 0) {
                        empty_mempool = true;
                        println!("txs is empty {}", txs.len());
                        continue;
                    }
                    
                    let mut contents: Vec<Content> = Vec::new();
    
                    //proposer
                    let proposer_content = ProposerContent {
                        parent_hash: locked_blockchain.get_proposer_tip(),
                        transactions: txs.clone(),
                        proposer_refs: locked_blockchain.get_unref_proposers(),
                    };
                    contents.push(block::Content::Proposer(proposer_content));
    
                    // Voters
                    let num_voter_chains = locked_blockchain.num_voter_chains;
                    for chain_num in 1..(num_voter_chains + 1) {
                        let tmp = VoterContent {
                            votes: locked_blockchain.get_votes(chain_num),
                            parent_hash: locked_blockchain.get_voter_tip(chain_num),
                            chain_num: chain_num,
                        };
                        contents.push(block::Content::Voter(tmp));
                    }
    
                    //drop(locked_blockchain);
    
                    let content_mkl_tree = MerkleTree::new(&contents);
    
                    let mut rng = rand::thread_rng();
                    let header = Header {
                        nonce: rng.gen::<u32>(),
                        difficulty: get_difficulty(num_voter_chains),
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(),
                        merkle_root: content_mkl_tree.root(),
                        miner_id: index as i32,   
                    };
    
                    let superblock = Superblock {
                        header: header,
                        content: contents,
                    };
    
                    let block_hash = superblock.hash();
                    // NOTE: Below works only for static difficulty
                    let difficulty = get_difficulty(num_voter_chains);
    
                    if block_hash < difficulty {
                        
                        // Sortition and decide the block index - proposer(0), voters(1..m)
                        let block_idx: u32 = sortition_hash(block_hash, difficulty, num_voter_chains).unwrap();
                        // println!("Mined a new block with {:?} hash", block_hash);
                        match &superblock.content[block_idx as usize] {
                            Content::Proposer(content) => {
                                println!("Mined a proposer with hash {:?} at index: {} and height {}",block_hash,block_idx,locked_blockchain.proposer_chain[&content.parent_hash].level+1);
                            }
                            Content::Voter(content) => {
                                println!("Mined a voter with hash {:?} at index: {} and height {}",block_hash,block_idx,locked_blockchain.voter_chains[(block_idx-1) as usize][&content.parent_hash].level+1);
                            }
                        }    
    
                        // Add header, relevant content and sortition proof
                        let sortition_proof = content_mkl_tree.proof(block_idx as usize);
                        let processed_block = Block {
                            header: superblock.header,
                            content: superblock.content[block_idx as usize].clone(),
                            sortition_proof: sortition_proof,
                        };
    
                        // Insert into local blockchain
                        // let mut locked_blockchain = self.blockchain.lock().unwrap();
                        locked_blockchain.insert(&processed_block);
                        // locked_blockchain.print_chains();
                        drop(locked_blockchain);
    
                        // Broadcast new block hash to the network
                        self.server.broadcast(Message::NewBlockHashes(vec![block_hash]));
                        
                        break;
                    }
                }
                // if (start.elapsed().as_secs() > experiment_duration) {
                //     info!("[Miner] finished experiment, shutting down ...");
                //     self.operating_state = OperatingState::ShutDown;
                //     break;
                // }
            }    
        }
    }
}
