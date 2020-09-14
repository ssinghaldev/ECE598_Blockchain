use super::hash::{Hashable, H256};
use std::collections::VecDeque;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    hashes: Vec<H256>,
    length: usize,
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let mut q:VecDeque<H256> = VecDeque::new();
        let mut tree:Vec<H256> = Vec::new();
        let mut datalen = data.len();

        for elem in data.into_iter() {
            q.push_back(elem.hash());
        }
        if datalen%2!=0 {
            q.push_back(data[datalen-1].hash());
            datalen = datalen + 1;
        }
        let mut count = 0;
        let mut levelen = datalen;
        while !q.is_empty() {
            let elem1 = q.pop_front().unwrap();
            tree.push(elem1);
            count = count + 1;
            let temp2 = q.pop_front();
            if temp2 == None {
                break;
            }
            let elem2 = temp2.unwrap();
            tree.push(elem2);
            count = count + 1;
            let leftchild = <[u8;32]>::from(elem1);
            let rightchild = <[u8;32]>::from(elem2);
            let parentval = [&leftchild[..],&rightchild[..]].concat();
            let parent:H256 = ring::digest::digest(&ring::digest::SHA256, &parentval[..]).into();
            q.push_back(parent);
            if count == levelen && count!=2 {
                let i = levelen/2;
                if i%2 != 0{
                    q.push_back(parent);
                    levelen = i+1;
                    count = 0;
                } else {
                    levelen = i;
                    count = 0;
                }
            }


        }

    MerkleTree{hashes:tree,length:datalen}
    }

    pub fn root(&self) -> H256 {
        self.hashes[self.hashes.len()-1]
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof:Vec<H256> = Vec::new();
        let mut i = index;
        let mut levelen = self.length;
        let mut divisor = 2;
        let mut sumexplored = 0;
        while levelen!=1 {
            if i%2 == 0 {
                proof.push(self.hashes[i+1]);
            }
            else {
                proof.push(self.hashes[i-1])
            }
            sumexplored = sumexplored + levelen;
            i = sumexplored + index/divisor;
            levelen = levelen/2;
            if levelen%2 == 1 && levelen!=1 {
                levelen = levelen + 1;
            }
            divisor = divisor*2;
        }
    proof
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, _leaf_size: usize) -> bool {
       let mut h = *datum;
       let mut res = false;
       let mut i = index;
       let mut iter = 0;
       while iter<proof.len() {
           let mut leftchild = <[u8;32]>::from(h);
           let mut rightchild = <[u8;32]>::from(h);
           
           if i%2 ==0 {
               //leftchild = <[u8;32]>::from(h);
               rightchild = <[u8;32]>::from(proof[iter]);
           } else {
               //rightchild = <[u8;32]>::from(h);
               leftchild = <[u8;32]>::from(proof[iter]);
           }

           let parentval = [&leftchild[..],&rightchild[..]].concat();
           h = ring::digest::digest(&ring::digest::SHA256, &parentval[..]).into();
           iter = iter + 1;
           i = i/2;
       }
       if h == *root {
           res = true;
       }

       res

}

#[cfg(test)]
mod tests {
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}
