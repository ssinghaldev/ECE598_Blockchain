use crate::crypto::hash::H256;
use crate::transaction::{SignedTransaction,UtxoInput};
use crate::crypto::hash::Hashable;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::BTreeMap;
use std::convert::TryInto;

use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::cmp;
  
#[derive(Debug)]
pub struct TransactionMempool{
    
    //counter for storage_index for btree
    counter: u32,

    //tx_hash to TxStore 
    hash_to_txstore:HashMap<H256,TxStore>,
    //map from tx_input to tx_hash (required for db check and  dependant tx removal)
    // to speed up duplicate removal
    // if a -> b, trans hash b is consuming utxoinput a
    // input_to_hash:HashMap<UtxoInput,H256>,
    // storage_index to txhash, used for maintaining FIFO order
    index_to_hash: BTreeMap<u32, H256>,
    utxoinputs: HashSet<H256>
}

#[derive(Debug, Clone)]
pub struct TxStore{  //used for storing a tx and its btree index

    pub signed_tx: SignedTransaction,
    
    //storage index for btree
    index: u32,

}
  
impl TransactionMempool{
    pub fn new() -> Self{
        TransactionMempool{ counter: 0,
            hash_to_txstore: HashMap::new(),
            index_to_hash: BTreeMap::new(), 
            utxoinputs: HashSet::new(),
        }
    }

    pub fn insert(&mut self, tx: SignedTransaction) {
            // println!("Size of mempool: {}", self.hash_to_txstore.len());
            println!("Received trans hash {} at {}", tx.hash(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros());

            let hash = tx.hash();
            for utxoinput in &tx.tx.tx_input {
                let utxoinput_hash = utxoinput.hash();
                if !(self.utxoinputs.insert(utxoinput_hash)) {
                    println!("Thief! {:?} trying to insert a douple spend in mempool", hash);
                }
            }
            
            let txstore = TxStore{
                signed_tx: tx,
                index: self.counter,
            };
            self.counter += 1;
            
            self.index_to_hash.insert(txstore.index, hash);
            self.hash_to_txstore.insert(hash, txstore);
    }

    // https://doc.rust-lang.org/std/option/
    // https://doc.rust-lang.org/edition-guide/rust-2018/error-handling-and-panics/the-question-mark-operator-for-easier-error-handling.html
    // ^ handy constructs for error handling

    pub fn get(&self, h: &H256) -> Option<&TxStore> {
        self.hash_to_txstore.get(h)
    }

    pub fn contains(&self, h: &H256) -> bool {
        self.hash_to_txstore.contains_key(h)
    }

    pub fn contains_utxoinput(&self, inputhash: &H256) -> bool {
        self.utxoinputs.contains(inputhash)
    }

    pub fn delete(&mut self, hash: &H256) -> bool {
        let txstore = self.hash_to_txstore.remove(hash);
        match txstore {
            Some(txstore) => {
                self.index_to_hash.remove(&txstore.index);
                true
            }
            None => {
                println!("Trying to delete non-existent hash");
                false
            }
        }
    }

    pub fn get_transactions(&self, n: u32) -> Vec<SignedTransaction> {
        let count = cmp::min(n, self.len().try_into().unwrap());
        self.index_to_hash.values().take(count as usize).map(|hash| self.get(hash).unwrap().signed_tx.clone()).collect()
    }
    
    pub fn len(&self) -> usize {
        self.hash_to_txstore.len()
    }

}
  
