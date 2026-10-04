//! Bounded-memory, transactional clustering state and recent-block rollback.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use fs2::FileExt;
use rusqlite::{params, Connection, OptionalExtension};

use crate::address::{CompactAddress, COMPACT_ADDRESS_SIZE};
use crate::block::MAINNET_MAGIC;
use crate::error::{ParseError, ParseResult};
use crate::hash::Hash;

const APPLICATION_ID: i64 = 0x42494f44;
const FORMAT_VERSION: i64 = 1;
const HEURISTIC_VERSION: i64 = 1;
pub const UNDO_BLOCKS: u64 = 144;

impl From<rusqlite::Error> for ParseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Chain(format!("Cluster database: {error}"))
    }
}

/// SQLite owns the bounded page cache; Rust retains only the current block's tip.
/// A directory lock prevents multiple analyzers from changing the same state.
pub struct ClusterStore {
    connection: Connection,
    _lock: File,
    active: Option<(u64, Hash)>,
}

struct Node {
    id: i64,
    parent: i64,
    rank: i64,
    canonical: i64,
}

fn invalid(message: &str) -> ParseError {
    ParseError::Chain(format!("Cluster database: {message}"))
}

fn height_value(height: u64) -> ParseResult<i64> {
    i64::try_from(height).map_err(|_| invalid("block height exceeds storage range"))
}

fn decode_hash(bytes: Vec<u8>) -> ParseResult<Hash> {
    Ok(Hash(
        bytes
            .try_into()
            .map_err(|_| invalid("invalid block hash"))?,
    ))
}

fn decode_address(bytes: Vec<u8>) -> ParseResult<CompactAddress> {
    let raw: [u8; COMPACT_ADDRESS_SIZE] = bytes
        .try_into()
        .map_err(|_| invalid("invalid address length"))?;
    CompactAddress::from_bytes(raw).ok_or_else(|| invalid("invalid compact address"))
}

impl ClusterStore {
    pub fn open(directory: &Path) -> ParseResult<Self> {
        Self::open_configured(directory, 1 << 30, 256 << 20)
    }

    /// Uses SQLite's process-wide hard heap limit; an already lower limit wins.
    /// The page cache uses at most three quarters of this budget, with disk spill.
    pub fn open_with_memory_limit(directory: &Path, sqlite_bytes: u64) -> ParseResult<Self> {
        Self::open_configured(directory, sqlite_bytes, sqlite_bytes / 4 * 3)
    }

    fn open_configured(directory: &Path, sqlite_bytes: u64, cache_bytes: u64) -> ParseResult<Self> {
        if !(1 << 20..=i64::MAX as u64).contains(&sqlite_bytes) {
            return Err(invalid(
                "SQLite memory budget must be between 1 MiB and i64::MAX bytes",
            ));
        }
        fs::create_dir_all(directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("cluster.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|error| invalid(&format!("cannot lock {}: {error}", directory.display())))?;
        let connection = Connection::open(directory.join("clusters.sqlite3"))?;
        // Bound startup inspection as well as replay: loading an existing schema
        // must not happen before SQLite has its process-wide allocation limit.
        connection.pragma_update(None, "hard_heap_limit", sqlite_bytes as i64)?;
        let application: i64 =
            connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let empty: bool = connection.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%')",
            [],
            |row| row.get(0),
        )?;
        if !(application == APPLICATION_ID && version == FORMAT_VERSION)
            && !(application == 0 && version == 0 && empty)
        {
            return Err(invalid("unrecognized or incompatible database format"));
        }
        // The cache limit is in KiB. SQLite spills dirty pages and temporary work
        // to disk rather than retaining a second copy of the working set in Rust.
        let actual_limit: i64 =
            connection.query_row("PRAGMA hard_heap_limit", [], |row| row.get(0))?;
        let cache_kib = cache_bytes.min(actual_limit as u64 / 4 * 3) / 1024;
        connection.pragma_update(None, "cache_size", -(cache_kib as i64))?;
        connection.execute_batch(
            "PRAGMA mmap_size=0;
             PRAGMA temp_store=FILE;
             PRAGMA cache_spill=ON;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA wal_autocheckpoint=1000;
             PRAGMA journal_size_limit=67108864;",
        )?;
        connection.set_prepared_statement_cache_capacity(32);
        if empty {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE metadata (
                    id INTEGER PRIMARY KEY CHECK(id=1),
                    network INTEGER NOT NULL,
                    heuristic INTEGER NOT NULL,
                    undo_depth INTEGER NOT NULL,
                    undo_floor INTEGER NOT NULL CHECK(undo_floor>=0)
                 );
                 CREATE TABLE blocks (
                    height INTEGER PRIMARY KEY CHECK(height>=0),
                    hash BLOB NOT NULL CHECK(length(hash)=32)
                 );
                 CREATE TABLE nodes (
                    id INTEGER PRIMARY KEY CHECK(id>0),
                    address BLOB NOT NULL UNIQUE CHECK(length(address)=42),
                    parent INTEGER NOT NULL CHECK(parent>0),
                    rank INTEGER NOT NULL CHECK(rank BETWEEN 0 AND 62),
                    canonical INTEGER NOT NULL CHECK(canonical>0)
                 );
                 CREATE TABLE utxos (
                    txid BLOB NOT NULL CHECK(length(txid)=32),
                    vout INTEGER NOT NULL CHECK(vout BETWEEN 0 AND 4294967295),
                    address BLOB NOT NULL CHECK(length(address)=42),
                    PRIMARY KEY(txid,vout)
                 ) WITHOUT ROWID;
                 CREATE TABLE undo_nodes (
                    height INTEGER NOT NULL,
                    id INTEGER NOT NULL,
                    parent INTEGER,
                    rank INTEGER,
                    canonical INTEGER,
                    PRIMARY KEY(height,id)
                 ) WITHOUT ROWID;
                 CREATE TABLE undo_utxos (
                    height INTEGER NOT NULL,
                    txid BLOB NOT NULL,
                    vout INTEGER NOT NULL,
                    address BLOB,
                    PRIMARY KEY(height,txid,vout)
                 ) WITHOUT ROWID;",
            )?;
            connection.execute(
                "INSERT INTO metadata VALUES(1,?1,?2,?3,0)",
                params![MAINNET_MAGIC as i64, HEURISTIC_VERSION, UNDO_BLOCKS as i64],
            )?;
            connection.pragma_update(None, "application_id", APPLICATION_ID)?;
            connection.pragma_update(None, "user_version", FORMAT_VERSION)?;
            connection.execute_batch("COMMIT")?;
            #[cfg(unix)]
            File::open(directory)?.sync_all()?;
        }
        let metadata: (i64, i64, i64) = connection.query_row(
            "SELECT network,heuristic,undo_depth FROM metadata WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if metadata != (MAINNET_MAGIC as i64, HEURISTIC_VERSION, UNDO_BLOCKS as i64) {
            return Err(invalid("incompatible network or clustering heuristic"));
        }
        Ok(Self {
            connection,
            _lock: lock,
            active: None,
        })
    }

    pub fn tip(&self) -> ParseResult<Option<(u64, Hash)>> {
        let tip: Option<(i64, Vec<u8>)> = self
            .connection
            .prepare_cached("SELECT height,hash FROM blocks ORDER BY height DESC LIMIT 1")?
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        tip.map(|(height, hash)| {
            Ok((
                u64::try_from(height).map_err(|_| invalid("negative block height"))?,
                decode_hash(hash)?,
            ))
        })
        .transpose()
    }

    pub fn hash_at(&self, height: u64) -> ParseResult<Option<Hash>> {
        let hash = self
            .connection
            .prepare_cached("SELECT hash FROM blocks WHERE height=?1")?
            .query_row([height_value(height)?], |row| row.get(0))
            .optional()?;
        hash.map(decode_hash).transpose()
    }

    pub fn utxo_count(&self) -> ParseResult<u64> {
        let count: i64 = self
            .connection
            .query_row("SELECT count(*) FROM utxos", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn begin_block(&mut self, height: u64, hash: Hash) -> ParseResult<()> {
        if self.active.is_some() {
            return Err(invalid("a block transaction is already active"));
        }
        height_value(height)?;
        let expected = match self.tip()? {
            Some((tip, _)) => tip
                .checked_add(1)
                .ok_or_else(|| invalid("block height overflow"))?,
            None => 0,
        };
        if height != expected {
            return Err(invalid("block does not immediately follow the saved tip"));
        }
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        self.active = Some((height, hash));
        Ok(())
    }

    fn active_height(&self) -> ParseResult<i64> {
        self.active
            .map(|(height, _)| height as i64)
            .ok_or_else(|| invalid("no block transaction is active"))
    }

    pub fn commit_block(&mut self) -> ParseResult<()> {
        let (height, hash) = self
            .active
            .ok_or_else(|| invalid("no block transaction is active"))?;
        self.connection
            .prepare_cached("INSERT INTO blocks(height,hash) VALUES(?1,?2)")?
            .execute(params![height as i64, hash.as_slice()])?;
        let floor = height.saturating_add(1).saturating_sub(UNDO_BLOCKS) as i64;
        self.connection
            .prepare_cached("DELETE FROM undo_nodes WHERE height<?1")?
            .execute([floor])?;
        self.connection
            .prepare_cached("DELETE FROM undo_utxos WHERE height<?1")?
            .execute([floor])?;
        self.connection
            .prepare_cached("UPDATE metadata SET undo_floor=MAX(undo_floor,?1) WHERE id=1")?
            .execute([floor])?;
        self.connection.execute_batch("COMMIT")?;
        self.active = None;
        Ok(())
    }

    pub fn abort_block(&mut self) -> ParseResult<()> {
        if !self.connection.is_autocommit() {
            self.connection.execute_batch("ROLLBACK")?;
        }
        self.active = None;
        Ok(())
    }

    /// Restores the selected ancestor, or clears state when its undo has expired.
    /// Callers must reread `tip()` because a deep reorganization restarts at genesis.
    pub fn rewind(&mut self, common_height: Option<u64>) -> ParseResult<()> {
        if self.active.is_some() {
            return Err(invalid("cannot rewind an active block"));
        }
        let Some((tip, _)) = self.tip()? else {
            return if common_height.is_none() {
                Ok(())
            } else {
                Err(invalid("ancestor is above the saved tip"))
            };
        };
        if common_height.is_some_and(|height| height > tip) {
            return Err(invalid("ancestor is above the saved tip"));
        }
        if common_height == Some(tip) {
            return Ok(());
        }
        let floor: i64 =
            self.connection
                .query_row("SELECT undo_floor FROM metadata WHERE id=1", [], |row| {
                    row.get(0)
                })?;
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> ParseResult<()> {
            if common_height.is_none_or(|height| height.saturating_add(1) < floor as u64) {
                self.connection.execute_batch(
                    "DELETE FROM utxos; DELETE FROM nodes; DELETE FROM blocks;
                     DELETE FROM undo_nodes; DELETE FROM undo_utxos;
                     UPDATE metadata SET undo_floor=0 WHERE id=1;",
                )?;
            } else {
                let ancestor = common_height.unwrap();
                for height in ((ancestor + 1)..=tip).rev() {
                    let height = height as i64;
                    self.connection
                        .prepare_cached(
                            "DELETE FROM utxos WHERE (txid,vout) IN
                         (SELECT txid,vout FROM undo_utxos WHERE height=?1)",
                        )?
                        .execute([height])?;
                    self.connection.prepare_cached(
                        "INSERT INTO utxos(txid,vout,address)
                         SELECT txid,vout,address FROM undo_utxos WHERE height=?1 AND address IS NOT NULL",
                    )?.execute([height])?;
                    self.connection.prepare_cached(
                        "DELETE FROM nodes WHERE id IN (SELECT id FROM undo_nodes WHERE height=?1 AND parent IS NULL)",
                    )?.execute([height])?;
                    self.connection.prepare_cached(
                        "UPDATE nodes SET (parent,rank,canonical)=
                         (SELECT parent,rank,canonical FROM undo_nodes WHERE height=?1 AND id=nodes.id)
                         WHERE id IN (SELECT id FROM undo_nodes WHERE height=?1 AND parent IS NOT NULL)",
                    )?.execute([height])?;
                }
                let ancestor = ancestor as i64;
                self.connection
                    .prepare_cached("DELETE FROM blocks WHERE height>?1")?
                    .execute([ancestor])?;
                self.connection
                    .prepare_cached("DELETE FROM undo_nodes WHERE height>?1")?
                    .execute([ancestor])?;
                self.connection
                    .prepare_cached("DELETE FROM undo_utxos WHERE height>?1")?
                    .execute([ancestor])?;
            }
            self.connection.execute_batch("COMMIT")?;
            Ok(())
        })();
        if result.is_err() && !self.connection.is_autocommit() {
            self.connection.execute_batch("ROLLBACK")?;
        }
        result
    }

    pub fn spend(&mut self, txid: &Hash, vout: u32) -> ParseResult<Option<CompactAddress>> {
        let height = self.active_height()?;
        let address: Option<Vec<u8>> = self
            .connection
            .prepare_cached("SELECT address FROM utxos WHERE txid=?1 AND vout=?2")?
            .query_row(params![txid.as_slice(), vout], |row| row.get(0))
            .optional()?;
        let Some(address) = address else {
            return Ok(None);
        };
        let address = decode_address(address)?;
        self.connection
            .prepare_cached(
                "INSERT OR IGNORE INTO undo_utxos(height,txid,vout,address)
             SELECT ?1,txid,vout,address FROM utxos WHERE txid=?2 AND vout=?3",
            )?
            .execute(params![height, txid.as_slice(), vout])?;
        self.connection
            .prepare_cached("DELETE FROM utxos WHERE txid=?1 AND vout=?2")?
            .execute(params![txid.as_slice(), vout])?;
        Ok(Some(address))
    }

    /// Replaces every surviving output with this txid, including historical duplicates.
    pub fn replace_outputs(
        &mut self,
        txid: &Hash,
        outputs: &[(u32, CompactAddress)],
    ) -> ParseResult<()> {
        let height = self.active_height()?;
        self.connection
            .prepare_cached(
                "INSERT OR IGNORE INTO undo_utxos(height,txid,vout,address)
             SELECT ?1,txid,vout,address FROM utxos WHERE txid=?2",
            )?
            .execute(params![height, txid.as_slice()])?;
        self.connection
            .prepare_cached("DELETE FROM utxos WHERE txid=?1")?
            .execute([txid.as_slice()])?;
        let mut undo = self.connection.prepare_cached(
            "INSERT OR IGNORE INTO undo_utxos(height,txid,vout,address) VALUES(?1,?2,?3,NULL)",
        )?;
        let mut insert = self
            .connection
            .prepare_cached("INSERT INTO utxos(txid,vout,address) VALUES(?1,?2,?3)")?;
        for (vout, address) in outputs {
            undo.execute(params![height, txid.as_slice(), vout])?;
            insert.execute(params![
                txid.as_slice(),
                vout,
                address.as_bytes().as_slice()
            ])?;
        }
        Ok(())
    }

    fn node(&self, id: i64) -> ParseResult<Node> {
        let node = self
            .connection
            .prepare_cached("SELECT parent,rank,canonical FROM nodes WHERE id=?1")?
            .query_row([id], |row| {
                Ok(Node {
                    id,
                    parent: row.get(0)?,
                    rank: row.get(1)?,
                    canonical: row.get(2)?,
                })
            })?;
        if node.id <= 0 || node.parent <= 0 || node.canonical <= 0 || !(0..=62).contains(&node.rank)
        {
            return Err(invalid("invalid union-find node"));
        }
        Ok(node)
    }

    // Union-by-rank bounds depth. Readonly traversal deliberately avoids path
    // compression: an export must never create unlogged changes to rollback state.
    fn root(&self, id: i64) -> ParseResult<Node> {
        let mut node = self.node(id)?;
        while node.parent != node.id {
            let parent = self.node(node.parent)?;
            if parent.rank <= node.rank {
                return Err(invalid("invalid union-find parent or cycle"));
            }
            node = parent;
        }
        Ok(node)
    }

    fn node_address(&self, id: i64) -> ParseResult<CompactAddress> {
        decode_address(
            self.connection
                .prepare_cached("SELECT address FROM nodes WHERE id=?1")?
                .query_row([id], |row| row.get(0))?,
        )
    }

    fn ensure_node(&self, address: &CompactAddress, height: i64) -> ParseResult<i64> {
        let existing: Option<i64> = self
            .connection
            .prepare_cached("SELECT id FROM nodes WHERE address=?1")?
            .query_row([address.as_bytes().as_slice()], |row| row.get(0))
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let last: i64 = self
            .connection
            .prepare_cached("SELECT COALESCE(MAX(id),0) FROM nodes")?
            .query_row([], |row| row.get(0))?;
        let id = last
            .checked_add(1)
            .ok_or_else(|| invalid("union-find node limit reached"))?;
        self.connection
            .prepare_cached(
                "INSERT INTO nodes(id,address,parent,rank,canonical) VALUES(?1,?2,?1,0,?1)",
            )?
            .execute(params![id, address.as_bytes().as_slice()])?;
        self.connection.prepare_cached(
            "INSERT OR IGNORE INTO undo_nodes(height,id,parent,rank,canonical) VALUES(?1,?2,NULL,NULL,NULL)",
        )?.execute(params![height, id])?;
        Ok(id)
    }

    fn log_node(&self, height: i64, id: i64) -> ParseResult<()> {
        self.connection
            .prepare_cached(
                "INSERT OR IGNORE INTO undo_nodes(height,id,parent,rank,canonical)
             SELECT ?1,id,parent,rank,canonical FROM nodes WHERE id=?2",
            )?
            .execute(params![height, id])?;
        Ok(())
    }

    pub fn merge(&mut self, inputs: &[CompactAddress]) -> ParseResult<()> {
        let height = self.active_height()?;
        if inputs.len() < 2 {
            return Ok(());
        }
        let mut last = self.ensure_node(&inputs[0], height)?;
        for address in &inputs[1..] {
            let next = self.ensure_node(address, height)?;
            let left = self.root(last)?;
            let right = self.root(next)?;
            if left.id != right.id {
                let canonical =
                    if self.node_address(left.canonical)? < self.node_address(right.canonical)? {
                        left.canonical
                    } else {
                        right.canonical
                    };
                let (winner, loser) = if left.rank > right.rank {
                    (left, right)
                } else {
                    (right, left)
                };
                self.log_node(height, winner.id)?;
                self.log_node(height, loser.id)?;
                self.connection
                    .prepare_cached("UPDATE nodes SET parent=?1 WHERE id=?2")?
                    .execute(params![winner.id, loser.id])?;
                let rank = winner.rank + i64::from(winner.rank == loser.rank);
                self.connection
                    .prepare_cached("UPDATE nodes SET rank=?1,canonical=?2 WHERE id=?3")?
                    .execute(params![rank, canonical, winner.id])?;
            }
            last = next;
        }
        Ok(())
    }

    /// Streams each address and its cluster's smallest address without mutating roots.
    pub fn write_csv(&self, writer: &mut impl Write) -> ParseResult<()> {
        if self.active.is_some() {
            return Err(invalid("cannot export an uncommitted block"));
        }
        let mut statement = self
            .connection
            .prepare_cached("SELECT id,address FROM nodes")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let address = decode_address(row.get(1)?)?;
            let representative = self.node_address(self.root(id)?.canonical)?;
            writeln!(writer, "{address},{representative}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash160::Hash160;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            Self(std::env::temp_dir().join(format!(
                "bitiodine-cluster-store-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn address(value: u8) -> CompactAddress {
        CompactAddress::from_hash160(&Hash160([value; 20]), 0)
    }

    fn hash(value: u64) -> Hash {
        Hash::from_data(&value.to_le_bytes())
    }

    fn csv(store: &ClusterStore) -> Vec<u8> {
        let mut bytes = Vec::new();
        store.write_csv(&mut bytes).unwrap();
        bytes
    }

    fn nodes(store: &ClusterStore) -> Vec<(i64, i64, i64, i64)> {
        store
            .connection
            .prepare("SELECT id,parent,rank,canonical FROM nodes ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn undo_restores_spends_duplicate_txids_union_roots_and_canonical_addresses() {
        let directory = TestDirectory::new();
        let mut store = ClusterStore::open(&directory.0).unwrap();
        store.begin_block(0, hash(0)).unwrap();
        store
            .replace_outputs(&hash(100), &[(0, address(3)), (1, address(2))])
            .unwrap();
        store.merge(&[address(3), address(2)]).unwrap();
        store.commit_block().unwrap();
        let original_csv = csv(&store);
        let original_nodes = nodes(&store);

        store.begin_block(1, hash(1)).unwrap();
        store
            .replace_outputs(&hash(101), &[(0, address(1))])
            .unwrap();
        assert_eq!(store.spend(&hash(100), 0).unwrap(), Some(address(3)));
        store.merge(&[address(3), address(1)]).unwrap();
        store
            .replace_outputs(&hash(100), &[(1, address(4)), (2, address(5))])
            .unwrap();
        assert_eq!(store.spend(&hash(100), 1).unwrap(), Some(address(4)));
        store
            .replace_outputs(&hash(100), &[(0, address(6))])
            .unwrap();
        store.merge(&[address(6), address(5)]).unwrap();
        store.commit_block().unwrap();
        assert_ne!(csv(&store), original_csv);
        let before_export = nodes(&store);
        assert!(!csv(&store).is_empty());
        assert_eq!(nodes(&store), before_export);
        drop(store);

        let mut store = ClusterStore::open(&directory.0).unwrap();
        assert_eq!(store.tip().unwrap(), Some((1, hash(1))));
        store.rewind(Some(0)).unwrap();
        assert_eq!(store.tip().unwrap(), Some((0, hash(0))));
        assert_eq!(csv(&store), original_csv);
        assert_eq!(nodes(&store), original_nodes);
        assert_eq!(store.utxo_count().unwrap(), 2);
        store.begin_block(1, hash(2)).unwrap();
        assert_eq!(store.spend(&hash(100), 0).unwrap(), Some(address(3)));
        assert_eq!(store.spend(&hash(100), 1).unwrap(), Some(address(2)));
        assert_eq!(store.spend(&hash(101), 0).unwrap(), None);
        store.abort_block().unwrap();
        assert_eq!(store.utxo_count().unwrap(), 2);
    }

    #[test]
    fn dropping_uncommitted_block_preserves_durable_tip_and_state() {
        let directory = TestDirectory::new();
        {
            let mut store = ClusterStore::open(&directory.0).unwrap();
            assert!(store.begin_block(1, hash(1)).is_err());
            assert!(store.replace_outputs(&hash(10), &[]).is_err());
            store.begin_block(0, hash(0)).unwrap();
            assert!(store.begin_block(0, hash(0)).is_err());
            store
                .replace_outputs(&hash(10), &[(0, address(1))])
                .unwrap();
            store.commit_block().unwrap();
            store.begin_block(1, hash(1)).unwrap();
            store.spend(&hash(10), 0).unwrap();
            store.merge(&[address(1), address(2)]).unwrap();
            // Connection close rolls back; simulates interrupted processing before commit.
        }
        let mut store = ClusterStore::open(&directory.0).unwrap();
        assert_eq!(store.tip().unwrap(), Some((0, hash(0))));
        assert_eq!(store.hash_at(1).unwrap(), None);
        assert_eq!(store.utxo_count().unwrap(), 1);
        assert!(csv(&store).is_empty());
        store.abort_block().unwrap();
        assert!(store.commit_block().is_err());
    }

    #[test]
    fn undo_retention_survives_rewind_and_deep_forks_restart_at_genesis() {
        let directory = TestDirectory::new();
        let mut store = ClusterStore::open(&directory.0).unwrap();
        for height in 0..=145 {
            store.begin_block(height, hash(height)).unwrap();
            store
                .replace_outputs(&hash(height + 1000), &[(0, address(1))])
                .unwrap();
            if height == 0 {
                store.merge(&[address(1), address(2)]).unwrap();
            }
            store.commit_block().unwrap();
        }
        // Exactly 144 blocks remain reversible, including heights with no node mutations.
        store.rewind(Some(1)).unwrap();
        assert_eq!(store.tip().unwrap(), Some((1, hash(1))));
        assert_eq!(store.utxo_count().unwrap(), 2);
        store.begin_block(2, hash(2000)).unwrap();
        store.commit_block().unwrap();
        // A new branch must not claim that already-pruned height 1 undo exists.
        store.rewind(Some(0)).unwrap();
        assert_eq!(store.tip().unwrap(), None);
        assert_eq!(store.utxo_count().unwrap(), 0);
        assert!(csv(&store).is_empty());
        store.begin_block(0, hash(3000)).unwrap();
        store.commit_block().unwrap();
        store.rewind(None).unwrap();
        assert_eq!(store.tip().unwrap(), None);
    }

    #[test]
    fn store_rejects_concurrent_access_and_incompatible_or_corrupt_formats() {
        let directory = TestDirectory::new();
        let store = ClusterStore::open(&directory.0).unwrap();
        assert!(ClusterStore::open(&directory.0).is_err());
        store
            .connection
            .pragma_update(None, "user_version", FORMAT_VERSION + 1)
            .unwrap();
        drop(store);
        assert!(ClusterStore::open(&directory.0).is_err());

        let directory = TestDirectory::new();
        let store = ClusterStore::open(&directory.0).unwrap();
        store
            .connection
            .execute("UPDATE metadata SET heuristic=999", [])
            .unwrap();
        drop(store);
        assert!(ClusterStore::open(&directory.0).is_err());

        let directory = TestDirectory::new();
        fs::create_dir_all(&directory.0).unwrap();
        fs::write(
            directory.0.join("clusters.sqlite3"),
            b"not a sqlite database",
        )
        .unwrap();
        assert!(ClusterStore::open(&directory.0).is_err());

        let directory = TestDirectory::new();
        fs::create_dir_all(&directory.0).unwrap();
        let connection = Connection::open(directory.0.join("clusters.sqlite3")).unwrap();
        connection
            .execute_batch("CREATE TABLE unrelated(value TEXT)")
            .unwrap();
        drop(connection);
        assert!(ClusterStore::open(&directory.0).is_err());
    }

    #[test]
    fn sqlite_memory_and_durability_settings_are_bounded() {
        let directory = TestDirectory::new();
        let store = ClusterStore::open(&directory.0).unwrap();
        let pragma = |name: &str| -> i64 {
            store
                .connection
                .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
                .unwrap()
        };
        assert!((-262144..0).contains(&pragma("cache_size")));
        assert!((1..=1 << 30).contains(&pragma("hard_heap_limit")));
        assert_eq!(pragma("mmap_size"), 0);
        assert_eq!(pragma("temp_store"), 1);
        assert!(pragma("cache_spill") > 0);
        assert_eq!(pragma("synchronous"), 2);
        let mode: String = store
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");

        let directory = TestDirectory::new();
        let limited = ClusterStore::open_with_memory_limit(&directory.0, 1 << 30).unwrap();
        let cache: i64 = limited
            .connection
            .query_row("PRAGMA cache_size", [], |row| row.get(0))
            .unwrap();
        let limit: i64 = limited
            .connection
            .query_row("PRAGMA hard_heap_limit", [], |row| row.get(0))
            .unwrap();
        assert!(limit > 0 && limit <= 1 << 30);
        assert_eq!(-cache, (limit / 4 * 3) / 1024);
    }

    #[test]
    fn startup_limit_is_applied_before_rejecting_foreign_database() {
        const CHILD_DIRECTORY: &str = "BITIODINE_STORE_STARTUP_LIMIT_CHILD";
        if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
            let limit = 32 << 20;
            assert!(ClusterStore::open_with_memory_limit(Path::new(&directory), limit).is_err());
            let connection = Connection::open_in_memory().unwrap();
            let actual: u64 = connection
                .query_row("PRAGMA hard_heap_limit", [], |row| row.get(0))
                .unwrap();
            assert_eq!(actual, limit);
            return;
        }
        let directory = TestDirectory::new();
        fs::create_dir_all(&directory.0).unwrap();
        let connection = Connection::open(directory.0.join("clusters.sqlite3")).unwrap();
        connection
            .pragma_update(None, "application_id", 1234)
            .unwrap();
        drop(connection);
        // Heap limits only decrease process-wide. Isolate the small test budget
        // so it cannot constrain unrelated unit tests running concurrently.
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cluster_store::tests::startup_limit_is_applied_before_rejecting_foreign_database",
            ])
            .env(CHILD_DIRECTORY, &directory.0)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }

    #[test]
    fn compact_address_codec_rejects_invalid_versions_lengths_and_padding() {
        let value = address(7);
        assert_eq!(CompactAddress::from_bytes(*value.as_bytes()), Some(value));
        let mut invalid = *value.as_bytes();
        invalid[41] = 1;
        assert!(CompactAddress::from_bytes(invalid).is_none());
        for (tag, length) in [(0x91, 32), (0xff, 32), (0x80, 2), (0x81, 41), (0x81, 1)] {
            let mut raw = [0; COMPACT_ADDRESS_SIZE];
            raw[0] = tag;
            raw[41] = length;
            assert!(CompactAddress::from_bytes(raw).is_none());
        }
        let mut witness = [0; COMPACT_ADDRESS_SIZE];
        witness[0] = 0x81;
        witness[41] = 2;
        assert!(CompactAddress::from_bytes(witness).is_some());
        witness[3] = 1;
        assert!(CompactAddress::from_bytes(witness).is_none());
    }

    #[test]
    fn corrupted_union_parent_does_not_loop_during_export() {
        let directory = TestDirectory::new();
        let mut store = ClusterStore::open(&directory.0).unwrap();
        store.begin_block(0, hash(0)).unwrap();
        store.merge(&[address(1), address(2)]).unwrap();
        store.commit_block().unwrap();
        store
            .connection
            .execute("UPDATE nodes SET parent=1 WHERE id=2", [])
            .unwrap();
        assert!(store.write_csv(&mut Vec::new()).is_err());
    }
}
