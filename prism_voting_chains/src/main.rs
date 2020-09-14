//#[cfg(test)]
#[macro_use]
extern crate hex_literal;

pub mod api;
pub mod block;
pub mod blockchain;
pub mod crypto;
pub mod miner;
pub mod network;
pub mod transaction;
pub mod mempool;
pub mod tx_generator;
pub mod validation;
pub mod ledger_manager;
pub mod utxo;

use clap::clap_app;
use crossbeam::channel;
use log::{error, debug,info};
use api::Server as ApiServer;
use network::{server, worker};
use std::net;
use std::process;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use crate::crypto::hash::{self, H256, Hashable};
use crate::block::{*};
use crate::utxo::{UtxoState};
use std::collections::HashSet;



fn main() {
    // parse command line arguments
    let matches = clap_app!(Bitcoin =>
     (version: "0.1")
     (about: "Bitcoin client")
     (@arg verbose: -v ... "Increases the verbosity of logging")
     (@arg peer_addr: --p2p [ADDR] default_value("127.0.0.1:6000") "Sets the IP address and the port of the P2P server")
     (@arg api_addr: --api [ADDR] default_value("127.0.0.1:7000") "Sets the IP address and the port of the API server")
     (@arg known_peer: -c --connect ... [PEER] "Sets the peers to connect to at start")
     (@arg p2p_workers: --("p2p-workers") [INT] default_value("4") "Sets the number of worker threads for P2P server")
     (@arg voter_chains: --("voter-chains") [INT] default_value("40") "Sets the number of voter chains")
     (@arg voter_depth_k: --("voter-depth-k") [INT] default_value("2") "Depth of votes before ledger manager can confirm")
    )
    .get_matches();

    // init logger
    let verbosity = matches.occurrences_of("verbose") as usize;
    stderrlog::new().verbosity(verbosity).init().unwrap();

    // parse p2p server address
    let p2p_addr = matches
        .value_of("peer_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P server address: {}", e);
            process::exit(1);
        });

    // parse api server address
    let api_addr = matches
        .value_of("api_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing API server address: {}", e);
            process::exit(1);
        });

    // create channels between server and worker
    let (msg_tx, msg_rx) = channel::unbounded();

    // start the p2p server
    let (server_ctx, server) = server::new(p2p_addr, msg_tx).unwrap();
    server_ctx.start().unwrap();

    //INTMOD
    let num_chains = matches
    .value_of("voter_chains")
    .unwrap()
    .parse::<u32>()
    .unwrap_or_else(|e| {
        error!("Error parsing voter chains: {}", e);
        process::exit(1);
    });

    info!("voter chains {}", num_chains);

    let voter_depth_k = matches
    .value_of("voter_depth_k")
    .unwrap()
    .parse::<u32>()
    .unwrap_or_else(|e| {
        error!("Error parsing voter depth k: {}", e);
        process::exit(1);
    });

    let utxo_state = Arc::new(Mutex::new(UtxoState::new()));

    // create mempool
    let mempool = Arc::new(Mutex::new(mempool::TransactionMempool::new()));

    // create blockchain
    let blockchain = Arc::new(Mutex::new(blockchain::Blockchain::new(num_chains, &mempool)));

    //create ledger_manager
    let ledger_manager = ledger_manager::LedgerManager::new(
        &blockchain,
        &utxo_state,
        voter_depth_k,
    );
    ledger_manager.start();

    // start the transaction generator
    // The transaction generator should not use blockchain
    let (txgen_ctx,txgen) = tx_generator::new(
        &server,
        &mempool,
        &utxo_state,
    );
    txgen_ctx.start(); 

  
     // start the miner
    let (miner_ctx, miner) = miner::new(
        &server,
        &blockchain,
        &mempool,
    );
    miner_ctx.start();

    // start the worker
    let p2p_workers = matches
        .value_of("p2p_workers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P workers: {}", e);
            process::exit(1);
        });
    let worker_ctx = worker::new(
        p2p_workers,
        msg_rx,
        &server,
        &blockchain,
        &mempool,
    );
    worker_ctx.start();

    



    // connect to known peers
    if let Some(known_peers) = matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        let server = server.clone();
        thread::spawn(move || {
            for peer in known_peers {
                loop {
                    let addr = match peer.parse::<net::SocketAddr>() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error parsing peer address {}: {}", &peer, e);
                            break;
                        }
                    };
                    match server.connect(addr) {
                        Ok(_) => {
                            info!("Connected to outgoing peer {}", &addr);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error connecting to peer {}, retrying in one second: {}",
                                addr, e
                            );
                            thread::sleep(time::Duration::from_millis(1000));
                            continue;
                        }
                    }
                }
            }
        });
    }


    // start the API server
    ApiServer::start(
        api_addr,
        &miner,
        &server,
        &txgen,
    );

    loop {
        std::thread::park();
    }
}
