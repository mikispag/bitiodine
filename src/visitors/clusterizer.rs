use log::info;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Write;

use foldhash::fast::RandomState;
use smallvec::SmallVec;

use crate::address::CompactAddress;
use crate::block::Block;
use crate::error::Result;
use crate::hash::ZERO_HASH;
use crate::hash160::Hash160;
use crate::script::{witness_program_version, HighLevel, Script};
use crate::transactions::{Transaction, TransactionInput, TransactionOutput};
use crate::visitors::BlockChainVisitor;

/// Header time from which BIP16 (P2SH) rules applied to a block.
pub const BIP16_ACTIVATION_TIME: u32 = 1_333_238_400;
/// First mainnet block height with BIP141 (SegWit) rules.
pub const SEGWIT_ACTIVATION_HEIGHT: u64 = 481_824;
/// First mainnet block height with BIP341 (Taproot) rules.
pub const TAPROOT_ACTIVATION_HEIGHT: u64 = 709_632;

/// Script rules in force in the block that spends an output.
///
/// Before a soft fork activated, outputs of its form could be spent without
/// the owner's key, so such spends are no evidence of common ownership.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpendRules {
    pub p2sh: bool,
    pub segwit: bool,
    pub taproot: bool,
}

impl SpendRules {
    /// Rules for a mainnet block with the given header time and height.
    pub fn at(timestamp: u32, height: u64) -> Self {
        Self {
            p2sh: timestamp >= BIP16_ACTIVATION_TIME,
            segwit: height >= SEGWIT_ACTIVATION_HEIGHT,
            taproot: height >= TAPROOT_ACTIVATION_HEIGHT,
        }
    }

    /// Whether spending an output paid to `address` with `script_sig` had to
    /// satisfy the output's key or redeem script under these rules.
    pub fn spend_is_authenticated(
        &self,
        address: &CompactAddress,
        script_sig: &Script<'_>,
    ) -> bool {
        match address.witness_program() {
            Some((0, _)) => self.segwit,
            Some((1, program)) if program.len() == 32 => self.taproot,
            // Other witness versions and lengths remain unencumbered.
            Some(_) => false,
            None if address.is_p2sh() => {
                if !self.p2sh {
                    return false;
                }
                match script_sig
                    .p2sh_redeem_script()
                    .and_then(witness_program_version)
                {
                    // P2SH-wrapped SegWit evaluates as true without a signature
                    // until SegWit activates.
                    Some(0) => self.segwit,
                    // Taproot rules do not apply to wrapped programs (BIP341).
                    Some(_) => false,
                    None => true,
                }
            }
            None => true,
        }
    }
}

pub struct Clusterizer {
    pub clusters: DisjointSet<CompactAddress>,
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
    pub input_addresses: SmallVec<[CompactAddress; 8]>,
    pub output_values: SmallVec<[u64; 4]>,
}

/// Tarjan's Union-Find data structure with union-by-rank and path compression.
pub struct DisjointSet<T: Clone + Hash + Eq> {
    set_size: u32,
    pub parent: Vec<u32>,
    pub rank: Vec<u8>,
    pub map: HashMap<T, u32, RandomState>, // Each T entry is mapped onto a u32 tag.
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
            map: HashMap::with_capacity_and_hasher(CAPACITY, Default::default()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        DisjointSet {
            set_size: 0,
            parent: Vec::with_capacity(capacity),
            rank: Vec::with_capacity(capacity),
            map: HashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    pub fn size(&self) -> usize {
        self.set_size as usize
    }

    pub fn is_empty(&self) -> bool {
        self.set_size == 0
    }

    /// Registers element x in the disjoint set if not present, returning its tag.
    pub fn make_set(&mut self, x: T) -> u32 {
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
    pub fn find(&mut self, x: &T) -> Option<u32> {
        let pos = *self.map.get(x)?;
        Some(Self::find_internal(&mut self.parent, pos))
    }

    /// Iterative two-pass path compression (prevents stack overflow).
    fn find_internal(p: &mut [u32], mut n: u32) -> u32 {
        let mut root = n;
        while root != p[root as usize] {
            root = p[root as usize];
        }
        while n != root {
            let parent = p[n as usize];
            p[n as usize] = root;
            n = parent;
        }
        root
    }

    /// Union the subsets to which x and y belong.
    /// Returns Some(tag) of the unified subset, or None if x or y is not in the set.
    pub fn union(&mut self, x: &T, y: &T) -> Option<u32> {
        let x_root = self.find(x)?;
        let y_root = self.find(y)?;

        if x_root == y_root {
            return Some(x_root);
        }

        let x_rank = self.rank[x_root as usize];
        let y_rank = self.rank[y_root as usize];

        if x_rank > y_rank {
            self.parent[y_root as usize] = x_root;
            Some(x_root)
        } else {
            self.parent[x_root as usize] = y_root;
            if x_rank == y_rank {
                self.rank[y_root as usize] = self.rank[y_root as usize].saturating_add(1);
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
    /// Streams clusters as CSV (`<address>,<cluster_representative_address>`) directly to a writer.
    /// Rows are written in map iteration order (unsorted); the second field is the
    /// lexicographically smallest address in the cluster (canonical representative),
    /// computed per root tag without hashing.
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let root_for_tag = |mut tag: u32| {
            while self.clusters.parent[tag as usize] != tag {
                tag = self.clusters.parent[tag as usize];
            }
            tag as usize
        };
        let mut root_to_min: Vec<Option<&CompactAddress>> = vec![None; self.clusters.parent.len()];
        for (address, &tag) in &self.clusters.map {
            let root = root_for_tag(tag);
            match &mut root_to_min[root] {
                Some(min) => {
                    if address < *min {
                        *min = address;
                    }
                }
                None => root_to_min[root] = Some(address),
            }
        }
        for (address, &tag) in &self.clusters.map {
            let root = root_for_tag(tag);
            let rep = root_to_min[root].expect("root tag must map to a cluster member");
            writeln!(writer, "{},{}", address, rep)?;
        }
        Ok(())
    }
}

/// Detects whether a transaction is a CoinJoin mixer transaction (e.g. Wasabi/Samourai/Whirlpool)
/// based on presence of 3 or more identical non-zero output amounts.
/// Sorts `output_values` in place; the slice is not used afterwards, so no heap
/// allocation or hashing is needed on this hot path.
pub(crate) fn is_coinjoin(output_values: &mut [u64]) -> bool {
    if output_values.len() < 3 {
        return false;
    }
    output_values.sort_unstable();
    let mut run = 1;
    for pair in output_values.windows(2) {
        if pair[0] == pair[1] && pair[0] != 0 {
            run += 1;
            if run >= 3 {
                return true;
            }
        } else {
            run = 1;
        }
    }
    false
}

pub(crate) fn output_address(txout: TransactionOutput<'_>) -> Option<CompactAddress> {
    match txout.script.to_highlevel() {
        HighLevel::PayToPubkey(pk) => {
            Some(CompactAddress::from_hash160(&Hash160::from_data(pk), 0x00))
        }
        HighLevel::PayToPubkeyHash(pkh) => {
            Some(CompactAddress::from_hash160(Hash160::from_slice(pkh), 0x00))
        }
        HighLevel::PayToScriptHash(pkh) => {
            Some(CompactAddress::from_hash160(Hash160::from_slice(pkh), 0x05))
        }
        HighLevel::PayToWitnessPubkeyHash(ref w)
        | HighLevel::PayToWitnessScriptHash(ref w)
        | HighLevel::PayToWitnessTaproot(ref w)
        | HighLevel::PayToWitnessGeneral(ref w) => Some(CompactAddress::from_witness_program(w)),
        _ => None,
    }
}

impl BlockChainVisitor for Clusterizer {
    type BlockItem = SpendRules;
    type TransactionItem = TransactionClusterState;
    type OutputItem = CompactAddress;
    type DoneItem = (usize, String);

    fn new() -> Self {
        Self {
            clusters: DisjointSet::new(),
        }
    }

    fn visit_block_begin<'a>(&mut self, block: Block<'a>, height: u64) -> SpendRules {
        SpendRules::at(block.header().timestamp(), height)
    }

    fn visit_transaction_begin(
        &mut self,
        _block_item: &mut Self::BlockItem,
    ) -> Self::TransactionItem {
        TransactionClusterState {
            input_addresses: SmallVec::new(),
            output_values: SmallVec::new(),
        }
    }

    fn visit_transaction_input<'a>(
        &mut self,
        txin: TransactionInput<'a>,
        rules: &mut Self::BlockItem,
        tx_item: &mut Self::TransactionItem,
        output_item: Option<Self::OutputItem>,
    ) {
        // Ignore coinbase
        if txin.prev_hash == &ZERO_HASH {
            return;
        }
        if let Some(address) = output_item {
            // A spend that needed no signature says nothing about ownership.
            if !rules.spend_is_authenticated(&address, &txin.script) {
                return;
            }
            if !tx_item.input_addresses.contains(&address) {
                tx_item.input_addresses.push(address);
            }
        }
    }

    fn visit_transaction_output<'a>(
        &mut self,
        txout: TransactionOutput<'a>,
        _block_item: &mut Self::BlockItem,
        transaction_item: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        transaction_item.output_values.push(txout.value);

        output_address(txout)
    }

    fn visit_transaction_end<'a>(
        &mut self,
        _tx: Transaction<'a>,
        _block_item: &mut Self::BlockItem,
        mut tx_item: Self::TransactionItem,
    ) {
        // Skip CoinJoin mixer transactions to avoid false-positive super-clusters
        if is_coinjoin(&mut tx_item.output_values) {
            return;
        }

        // Merge inputs according to multi-input clustering heuristic
        if tx_item.input_addresses.len() > 1 {
            let mut tx_inputs_iter = tx_item.input_addresses.into_iter();
            let mut last_address = tx_inputs_iter.next().unwrap();
            self.clusters.make_set(last_address);
            for address in tx_inputs_iter {
                self.clusters.make_set(address);
                let _ = self.clusters.union(&last_address, &address);
                last_address = address;
            }
        }
    }

    fn done(&mut self) -> Result<(usize, String)> {
        self.clusters.finalize();
        info!("{} addresses clustered.", self.clusters.size());
        Ok((self.clusters.size(), String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::is_coinjoin;

    #[test]
    fn test_is_coinjoin() {
        assert!(!is_coinjoin(&mut []));
        assert!(!is_coinjoin(&mut [1, 1]));
        assert!(!is_coinjoin(&mut [0, 0, 0])); // zero-value outputs never count
        assert!(!is_coinjoin(&mut [1, 1, 2, 2]));
        assert!(is_coinjoin(&mut [100, 100, 100]));
        assert!(is_coinjoin(&mut [3, 0, 3, 1, 3, 2])); // unsorted, zeros mixed in
    }
}
