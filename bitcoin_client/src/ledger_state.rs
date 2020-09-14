use crate::transaction::{UtxoInput, UtxoOutput};
use crate::crypto::hash::H256;
use crate::block::Block;
use crate::crypto::hash::Hashable;
use crate::crypto::address::{self, H160};

use std::collections::HashMap;
use log::debug;

#[derive(Debug, Default, Clone)]
pub struct State{
    pub state_map: HashMap<UtxoInput, UtxoOutput>,  
}

pub struct BlockState{
    pub block_state_map: HashMap<H256, State>,
}

//TODO: Can change the signature of the function if passing some mutex version
//of block_state
pub fn update_block_state(block: &Block, block_state: &mut BlockState) {
  let parent_state = &block_state.block_state_map[&block.header.parenthash];
  let mut cur_block_state = parent_state.clone();
  
  debug!{"parent_state {:?}", parent_state}

  for signed_tx in &block.content.data {
      for tx_input in &signed_tx.tx.tx_input {
          cur_block_state.state_map.remove(tx_input);
      }
      for (i, tx_output) in (&signed_tx.tx.tx_output).iter().enumerate() {
          let tx_input = UtxoInput{tx_hash: signed_tx.tx.hash(), idx: i as u8};
          cur_block_state.state_map.insert(tx_input, tx_output.clone());
      }
  }
  
  debug!{"cur_block_state {:?}", cur_block_state}
  block_state.block_state_map.insert(block.hash(), cur_block_state);
}

pub fn ico() -> State {
  let public_key1: Vec<u8> = b"AAAAC3NzaC1lZDI1NTE5AAAAICYqyx/qrxvVPB2lPvV3ZmTH+uYwB6wL1hkBlGaYPmGu".to_vec();
  let public_key2: Vec<u8> = b"AAAAC3NzaC1lZDI1NTE5AAAAIDfqgH+ezyswXrz2YNDkkYXCTCTMi+Ms6GWW5NQXNUc4".to_vec();
  let public_key3: Vec<u8> = b"AAAAC3NzaC1lZDI1NTE5AAAAIMborH2X51+g+ziV0LmZY8p90+eEP/9jPAOUauBPorL/".to_vec();
  
  let mut address_vec: Vec<H160> = vec![];
  let address1 = address::address_from_public_key_vec_ref(&public_key1);
  let address2 = address::address_from_public_key_vec_ref(&public_key2);
  let address3 = address::address_from_public_key_vec_ref(&public_key3);
  
  address_vec.push(address1);
  address_vec.push(address2);
  address_vec.push(address3);

  let initial_tx_hash: H256 = hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920").into() ;
  let val: u32 = 10000000;
  
  let mut initial_state: State = State{state_map: HashMap::new()};
  for (i,address) in  address_vec.iter().enumerate() {
    let input = UtxoInput{tx_hash: initial_tx_hash, idx: i as u8};
    let output = UtxoOutput{receipient_addr: *address, value: val};
    initial_state.state_map.insert(input, output);
  }

  initial_state
}
