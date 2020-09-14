use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::block::*;
use crate::ledger_manager::LedgerManager;
use crate::transaction::{self,*};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::address::{*};
use crate::network::message::Message;
use log::{debug,info};
use rand::Rng;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use ring::signature::{self,Ed25519KeyPair, Signature, KeyPair};
use crate::mempool::TransactionMempool;
use crate::crypto::key_pair;
use crate::crypto::address::{self,*};
use std::borrow::Borrow;
use std::collections::{HashSet, HashMap};
use crate::utxo::{UtxoState};


use rand::seq::SliceRandom;

extern crate chrono;
use chrono::prelude::*;

use std::time;
use std::thread;
use std::sync::{Arc, Mutex};

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
    mempool: Arc<Mutex<TransactionMempool>>,
    utxo_state: Arc<Mutex<UtxoState>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    mempool: &Arc<Mutex<TransactionMempool>>,
    utxo_state: &Arc<Mutex<UtxoState>>,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        mempool: Arc::clone(mempool),
        utxo_state: Arc::clone(utxo_state),
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
            .name("tx_generator".to_string())
            .spawn(move || {
                self.gen_loop();
            })
            .unwrap();
        info!("Generator initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                println!("Generator shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i,j) => {
                println!("Generator starting in continuous mode with lambda {} and index {}", i,j);
                self.operating_state = OperatingState::Run(i,j);
            }
        }
    }

    fn gen_loop(&mut self) {

        let vector1 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 187, 131, 74, 161, 134, 11, 240, 6, 188, 109, 18, 108, 124, 219, 167, 164, 215, 125, 168, 79, 204, 194, 232, 91, 58, 186, 181, 230, 212, 78, 163, 28, 161, 35, 3, 33, 0, 233, 72, 146, 218, 220, 235, 17, 123, 202, 112, 119, 63, 134, 105, 134, 71, 34, 185, 71, 193, 59, 66, 43, 137, 50, 194, 120, 234, 97, 132, 235, 159];
        let vector2 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 154, 186, 73, 239, 105, 129, 142, 211, 156, 79, 213, 209, 229, 87, 22, 92, 113, 203, 244, 222, 244, 33, 199, 254, 130, 102, 178, 65, 198, 67, 20, 132, 161, 35, 3, 33, 0, 161, 153, 171, 27, 96, 146, 25, 237, 5, 189, 186, 116, 0, 24, 2, 8, 28, 143, 5, 119, 20, 47, 142, 186, 55, 234, 189, 167, 154, 15, 210, 97];
        let vector3 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 147, 195, 231, 118, 135, 29, 32, 40, 23, 117, 107, 218, 6, 220, 198, 50, 81, 113, 167, 122, 175, 161, 118, 93, 191, 137, 50, 125, 203, 69, 70, 42, 161, 35, 3, 33, 0, 125, 80, 160, 138, 247, 46, 227, 162, 118, 51, 64, 42, 174, 60, 87, 134, 77, 60, 225, 11, 189, 222, 22, 185, 65, 10, 67, 78, 250, 41, 188, 60];
        let vector4 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 11, 212, 170, 1, 126, 8, 32, 58, 40, 116, 165, 98, 48, 127, 67, 109, 86, 251, 249, 203, 244, 203, 1, 223, 248, 164, 176, 195, 23, 17, 146, 8, 161, 35, 3, 33, 0, 206, 15, 234, 106, 58, 45, 177, 81, 0, 193, 13, 113, 249, 55, 152, 151, 227, 224, 35, 185, 148, 49, 186, 234, 17, 106, 132, 216, 83, 196, 127, 99];
        let vector5 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 40, 29, 27, 179, 25, 183, 68, 113, 252, 19, 20, 114, 160, 221, 228, 195, 253, 87, 245, 176, 226, 99, 249, 28, 87, 61, 101, 129, 207, 87, 90, 195, 161, 35, 3, 33, 0, 254, 57, 159, 24, 159, 141, 184, 159, 58, 86, 112, 217, 153, 215, 65, 7, 88, 14, 57, 80, 42, 33, 151, 211, 208, 52, 42, 208, 111, 174, 223, 27];
        let vector6 = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 224, 231, 169, 219, 160, 221, 218, 51, 189, 197, 202, 218, 24, 20, 166, 105, 31, 55, 241, 231, 5, 165, 51, 106, 174, 11, 110, 84, 17, 115, 230, 56, 161, 35, 3, 33, 0, 127, 130, 60, 237, 224, 179, 64, 241, 25, 174, 45, 64, 52, 179, 70, 249, 26, 49, 128, 103, 188, 201, 48, 55, 221, 154, 12, 83, 40, 123, 3, 157];

        let key1 = signature::Ed25519KeyPair::from_pkcs8(vector1.as_ref().into()).unwrap();
        let key2 = signature::Ed25519KeyPair::from_pkcs8(vector2.as_ref().into()).unwrap();
        let key3 = signature::Ed25519KeyPair::from_pkcs8(vector3.as_ref().into()).unwrap();
        let key4 = signature::Ed25519KeyPair::from_pkcs8(vector4.as_ref().into()).unwrap();
        let key5 = signature::Ed25519KeyPair::from_pkcs8(vector5.as_ref().into()).unwrap();
        let key6 = signature::Ed25519KeyPair::from_pkcs8(vector6.as_ref().into()).unwrap();

        let address1 = address::address_from_public_key_vec_ref(&key1.public_key().as_ref().to_vec());
        let address2 = address::address_from_public_key_vec_ref(&key2.public_key().as_ref().to_vec());
        let address3 = address::address_from_public_key_vec_ref(&key3.public_key().as_ref().to_vec());
        let address4 = address::address_from_public_key_vec_ref(&key4.public_key().as_ref().to_vec());
        let address5 = address::address_from_public_key_vec_ref(&key5.public_key().as_ref().to_vec());
        let address6 = address::address_from_public_key_vec_ref(&key6.public_key().as_ref().to_vec());

        let mut address_vec: Vec<H160> = Vec::new();
        address_vec.push(address1);
        address_vec.push(address2);
        address_vec.push(address3);
        address_vec.push(address4);
        address_vec.push(address5);
        address_vec.push(address6);


        let mut index:u64 = 0;
        let mut time_i:u64 = 0;

        //let mut generated_txs: HashSet<H256> = HashSet::new();

        loop {
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
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            if let OperatingState::Run(i,j) = self.operating_state {
                index = j;
                time_i =i;  
            }

            let mut locked_mempool = self.mempool.lock().unwrap();
            if locked_mempool.len() >= 15 {
                if time_i != 0 {
                    drop(locked_mempool);
                    let interval = time::Duration::from_micros(time_i);
                    thread::sleep(interval);
                    continue;
                }
            }

            let locked_utxostate = self.utxo_state.lock().unwrap();
            let mut tx_buffer : Vec<H256> = vec![];

            println!("Current number of utxo entries {}", locked_utxostate.state_map.len());
            println!("locked mempool size {}", locked_mempool.len());
            
            let mut responsible_addresses:Vec<H160> = Vec::new();
            match index {
                0 => {responsible_addresses.push(address1);responsible_addresses.push(address2)},
                1 => {responsible_addresses.push(address3);responsible_addresses.push(address4)},
                2 => {responsible_addresses.push(address5);responsible_addresses.push(address6)},
                _ => println!("Invalid index"),
            }

            for (input, output) in locked_utxostate.state_map.iter() {
                if (locked_mempool.contains_utxoinput(&input.hash())) {
                    continue;
                }

                if !responsible_addresses.iter().any(|&i| i==output.receipient_addr) {
                    continue;
                }

                let mut vec_input:Vec<UtxoInput> = vec![]; 
                let mut vec_output:Vec<UtxoOutput> = vec![];

                vec_input.push(input.clone());

                let mut new_output = output.clone();
                let new_receipient = *address_vec.choose(&mut rand::thread_rng()).unwrap();
                new_output.receipient_addr = new_receipient;

                vec_output.push(new_output);

                let raw_tx = Transaction {
                    tx_input: vec_input,
                    tx_output: vec_output,
                };

                // assign dummy values for now, this is changed inside match scope
                // Rust does not allow changing the data type of a variable
                let mut signature = sign(&raw_tx, &key1).as_ref().to_vec();
                let mut public_key = key1.public_key().as_ref().to_vec();

                let old_receipient = output.receipient_addr;
                if (old_receipient == address1) {
                    signature = sign(&raw_tx, &key1).as_ref().to_vec();
                    public_key = key1.public_key().as_ref().to_vec();
                    // println!("Sending address1's coin");
                } else if (old_receipient == address2) {
                    signature = sign(&raw_tx, &key2).as_ref().to_vec();
                    public_key = key2.public_key().as_ref().to_vec();
                    // println!("Sending address2's coin");
                } else if (old_receipient == address3) {
                    signature = sign(&raw_tx, &key3).as_ref().to_vec();
                    public_key = key3.public_key().as_ref().to_vec();
                    // println!("Sending address3's coin");
                } else if (old_receipient == address4) {
                    signature = sign(&raw_tx, &key4).as_ref().to_vec();
                    public_key = key4.public_key().as_ref().to_vec();
                    // println!("Sending address4's coin");
                } else if (old_receipient == address5) {
                    signature = sign(&raw_tx, &key5).as_ref().to_vec();
                    public_key = key5.public_key().as_ref().to_vec();
                    // println!("Sending address5's coin");
                } else if (old_receipient == address6) {
                    signature = sign(&raw_tx, &key6).as_ref().to_vec();
                    public_key = key6.public_key().as_ref().to_vec();
                    // println!("Sending address6's coin");
                } else {
                    println!("I am only aware of six addresses, I don't know you!!!");
                }

                let signed_tx = SignedTransaction {
                    tx: raw_tx,
                    signature: signature,
                    public_key: public_key,
                };

                if locked_mempool.contains(&signed_tx.hash()){
                    continue;
                } else {
                    tx_buffer.push(signed_tx.hash());
                    if tx_buffer.len() > 5 {
                        break;
                    }
                    locked_mempool.insert(signed_tx);
                }
                
            }

            drop(locked_utxostate);

            if tx_buffer.len() > 0 {
                self.server.broadcast(Message::NewTransactionHashes(tx_buffer));
            }

            drop(locked_mempool);

            if time_i != 0 {
                let interval = time::Duration::from_micros(time_i);
                thread::sleep(interval);
            }
        }
    }
}