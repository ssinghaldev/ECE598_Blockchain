use crate::crypto::hash::H256;
use crate::transaction::SignedTransaction;

use std::collections::VecDeque;
use std::collections::HashMap;

pub struct TransactionMempool{
  pub tx_hash_queue: VecDeque<H256>,
  pub tx_to_process: HashMap<H256, bool>,
  pub tx_map: HashMap<H256, SignedTransaction>,
}

impl TransactionMempool{
  pub fn new() -> Self{
    TransactionMempool{tx_hash_queue: VecDeque::new(), 
                       tx_to_process: HashMap::new(), 
                       tx_map: HashMap::new()}  
  }
}
