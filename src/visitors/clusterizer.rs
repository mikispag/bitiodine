use log::info;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::Write;

use crate::address::Address;
use crate::block::Block;
use crate::error::Result;
use crate::hash::ZERO_HASH;
use crate::hash160::Hash160;
use crate::script::HighLevel;
use crate::transactions::{Transaction, TransactionInput, TransactionOutput};
use crate::visitors::BlockChainVisitor;

pub struct Clusterizer {
    pub clusters: DisjointSet<Address>,
}

/// Tarjan's Union-Find data structure with union-by-rank and path compression.
pub struct DisjointSet<T: Clone + Hash + Eq> {
    set_size: usize,
    pub parent: Vec<usize>,
    pub rank: Vec<usize>,
    pub map: HashMap<T, usize>, // Each T entry is mapped onto a usize tag.
}

impl<T> Default for DisjointSet<T>
where
    T: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DisjointSet<T>
where
    T: Clone + Hash + Eq,
{
    pub fn new() -> Self {
        const CAPACITY: usize = 1_000_000;
        DisjointSet {
            set_size: 0,
            parent: Vec::with_capacity(CAPACITY),
            rank: Vec::with_capacity(CAPACITY),
            map: HashMap::with_capacity(CAPACITY),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        DisjointSet {
            set_size: 0,
            parent: Vec::with_capacity(capacity),
            rank: Vec::with_capacity(capacity),
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn size(&self) -> usize {
        self.set_size
    }

    pub fn is_empty(&self) -> bool {
        self.set_size == 0
    }

    pub fn make_set(&mut self, x: T) {
        if self.map.contains_key(&x) {
            return;
        }

        let len = self.set_size;
        self.map.insert(x, len);
        self.parent.push(len);
        self.rank.push(0);

        self.set_size += 1;
    }

    /// Returns Some(tag), the root tag of the subset containing x.
    /// If x is not in the data structure, returns None.
    pub fn find(&mut self, x: &T) -> Option<usize> {
        let pos = *self.map.get(x)?;
        Some(Self::find_internal(&mut self.parent, pos))
    }

    /// Iterative two-pass path compression (prevents stack overflow).
    fn find_internal(p: &mut [usize], mut n: usize) -> usize {
        let mut root = n;
        while root != p[root] {
            root = p[root];
        }
        while n != root {
            let parent = p[n];
            p[n] = root;
            n = parent;
        }
        root
    }

    /// Union the subsets to which x and y belong.
    /// Returns Some(tag) of the unified subset, or None if x or y is not in the set.
    pub fn union(&mut self, x: &T, y: &T) -> Option<usize> {
        let x_root = self.find(x)?;
        let y_root = self.find(y)?;

        if x_root == y_root {
            return Some(x_root);
        }

        let x_rank = self.rank[x_root];
        let y_rank = self.rank[y_root];

        if x_rank > y_rank {
            self.parent[y_root] = x_root;
            Some(x_root)
        } else {
            self.parent[x_root] = y_root;
            if x_rank == y_rank {
                self.rank[y_root] += 1;
            }
            Some(y_root)
        }
    }

    /// Forces all laziness, updating every tag to its canonical root.
    pub fn finalize(&mut self) {
        for i in 0..self.set_size {
            Self::find_internal(&mut self.parent, i);
        }
    }
}

impl Clusterizer {
    /// Writes clusters as CSV (`<address>,<cluster_id>`) directly to a writer.
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        for (address, tag) in &self.clusters.map {
            writeln!(writer, "{},{}", address, self.clusters.parent[*tag])?;
        }
        Ok(())
    }
}

impl<'a> BlockChainVisitor<'a> for Clusterizer {
    type BlockItem = ();
    type TransactionItem = HashSet<Address>;
    type OutputItem = Address;
    type DoneItem = (usize, String);

    fn new() -> Self {
        Self {
            clusters: DisjointSet::new(),
        }
    }

    fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(
        &mut self,
        _block_item: &mut Self::BlockItem,
    ) -> Self::TransactionItem {
        HashSet::with_capacity(100)
    }

    fn visit_transaction_input(
        &mut self,
        txin: TransactionInput<'a>,
        _block_item: &mut Self::BlockItem,
        tx_item: &mut Self::TransactionItem,
        output_item: Option<Self::OutputItem>,
    ) {
        // Ignore coinbase
        if txin.prev_hash == &ZERO_HASH {
            return;
        }
        if let Some(address) = output_item {
            tx_item.insert(address);
        }
    }

    fn visit_transaction_output(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        _transaction_item: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => {
                Some(Address::from_hash160(Hash160::from_slice(pkh), 0x00))
            }
            HighLevel::PayToScriptHash(pkh) => {
                Some(Address::from_hash160(Hash160::from_slice(pkh), 0x05))
            }
            HighLevel::PayToWitnessPubkeyHash(w) | HighLevel::PayToWitnessScriptHash(w) => {
                Some(Address(w.to_address()))
            }
            _ => None,
        }
    }

    fn visit_transaction_end(
        &mut self,
        _tx: Transaction<'a>,
        _block_item: &mut Self::BlockItem,
        tx_item: Self::TransactionItem,
    ) {
        // Skip transactions with just one input
        if tx_item.len() > 1 {
            let mut tx_inputs_iter = tx_item.iter();
            let mut last_address = tx_inputs_iter.next().unwrap();
            self.clusters.make_set(last_address.clone());
            for address in tx_inputs_iter {
                self.clusters.make_set(address.clone());
                let _ = self.clusters.union(last_address, address);
                last_address = address;
            }
        }
    }

    fn done(&mut self) -> Result<(usize, String)> {
        self.clusters.finalize();

        let mut output_string = String::new();
        for (address, tag) in &self.clusters.map {
            output_string.push_str(&format!("{},{}\n", address, self.clusters.parent[*tag]));
        }

        info!("{} addresses clustered.", self.clusters.size());
        Ok((self.clusters.size(), output_string))
    }
}
