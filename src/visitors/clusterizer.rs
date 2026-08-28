use log::info;
use std::collections::hash_map::Entry;
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

impl Default for Clusterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clusterizer {
    pub fn new() -> Self {
        Self {
            clusters: DisjointSet::new(),
        }
    }
}

pub struct TransactionClusterState {
    pub input_addresses: HashSet<Address>,
    pub output_values: Vec<u64>,
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

    /// Registers element x in the disjoint set if not present, returning its tag.
    pub fn make_set(&mut self, x: T) -> usize {
        match self.map.entry(x) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let len = self.set_size;
                entry.insert(len);
                self.parent.push(len);
                self.rank.push(0);
                self.set_size += 1;
                len
            }
        }
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
    /// Builds a deterministic mapping from each disjoint-set root tag to the lexicographically smallest address in that cluster.
    pub fn canonical_representatives(&self) -> HashMap<usize, &Address> {
        let mut root_to_min: HashMap<usize, &Address> =
            HashMap::with_capacity(self.clusters.size().min(self.clusters.map.len()));
        for (address, &tag) in &self.clusters.map {
            let root = self.clusters.parent[tag];
            root_to_min
                .entry(root)
                .and_modify(|min| {
                    if address < *min {
                        *min = address;
                    }
                })
                .or_insert(address);
        }
        root_to_min
    }

    /// Writes clusters as CSV (`<address>,<cluster_representative_address>`) directly to a writer,
    /// sorted deterministically by address.
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let root_to_min = self.canonical_representatives();

        let mut entries: Vec<(&Address, &Address)> = self
            .clusters
            .map
            .iter()
            .map(|(address, &tag)| {
                let root = self.clusters.parent[tag];
                (address, root_to_min[&root])
            })
            .collect();

        entries.sort_unstable_by_key(|&(addr, _)| addr);

        for (address, rep) in entries {
            writeln!(writer, "{},{}", address, rep)?;
        }
        Ok(())
    }
}

/// Detects whether a transaction is a CoinJoin mixer transaction (e.g. Wasabi/Samourai/Whirlpool)
/// based on presence of 3 or more identical non-zero output amounts.
fn is_coinjoin(output_values: &[u64]) -> bool {
    if output_values.len() < 3 {
        return false;
    }
    let mut counts: HashMap<u64, usize> = HashMap::with_capacity(output_values.len());
    for &val in output_values {
        if val > 0 {
            let count = counts.entry(val).or_insert(0);
            *count += 1;
            if *count >= 3 {
                return true;
            }
        }
    }
    false
}

impl<'a> BlockChainVisitor<'a> for Clusterizer {
    type BlockItem = ();
    type TransactionItem = TransactionClusterState;
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
        TransactionClusterState {
            input_addresses: HashSet::with_capacity(32),
            output_values: Vec::with_capacity(16),
        }
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
            tx_item.input_addresses.insert(address);
        }
    }

    fn visit_transaction_output(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut (),
        transaction_item: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        transaction_item.output_values.push(txout.value);

        match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => {
                Some(Address::from_hash160(Hash160::from_slice(pkh), 0x00))
            }
            HighLevel::PayToScriptHash(pkh) => {
                Some(Address::from_hash160(Hash160::from_slice(pkh), 0x05))
            }
            HighLevel::PayToWitnessPubkeyHash(w)
            | HighLevel::PayToWitnessScriptHash(w)
            | HighLevel::PayToWitnessTaproot(w)
            | HighLevel::PayToWitnessGeneral(w) => Some(Address(w.to_address())),
            _ => None,
        }
    }

    fn visit_transaction_end(
        &mut self,
        _tx: Transaction<'a>,
        _block_item: &mut Self::BlockItem,
        tx_item: Self::TransactionItem,
    ) {
        // Skip CoinJoin mixer transactions to avoid false-positive super-clusters
        if is_coinjoin(&tx_item.output_values) {
            return;
        }

        // Merge inputs according to multi-input clustering heuristic
        if tx_item.input_addresses.len() > 1 {
            let mut tx_inputs_iter = tx_item.input_addresses.into_iter();
            let mut last_address = tx_inputs_iter.next().unwrap();
            self.clusters.make_set(last_address.clone());
            for address in tx_inputs_iter {
                self.clusters.make_set(address.clone());
                let _ = self.clusters.union(&last_address, &address);
                last_address = address;
            }
        }
    }

    fn done(&mut self) -> Result<(usize, String)> {
        self.clusters.finalize();

        let mut output_bytes = Vec::new();
        self.write_csv(&mut output_bytes)
            .map_err(|_| crate::error::EofError)?;
        let output_string = String::from_utf8(output_bytes).unwrap();

        info!("{} addresses clustered.", self.clusters.size());
        Ok((self.clusters.size(), output_string))
    }
}
