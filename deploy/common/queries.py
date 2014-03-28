in_query_addr = """
	  SELECT
			  txout.address
			  FROM txin
			  LEFT JOIN txout ON (txout.txout_id = txin.txout_id)
			 WHERE txin.tx_id = ?
"""
out_query_addr = """
	  SELECT
			  txout.address
			  FROM txout
			  LEFT JOIN txin ON (txin.txout_id = txout.txout_id)
			 WHERE txout.tx_id = ?
"""
tx_hash_query = """
	  SELECT
			  tx_hash
			  FROM tx
			  WHERE tx_id = ?
"""
out_query_addr_with_value = """
	  SELECT
			  txout.address, txout.txout_value
			  FROM txout
			  LEFT JOIN txin ON (txin.txout_id = txout.txout_id)
			 WHERE txout.tx_id = ?
"""
time_query = """
	  SELECT
			  time
			  FROM tx
			  LEFT JOIN blocks ON (tx.block_id = blocks.block_id)
			 WHERE tx.tx_id = ?
"""

number_of_transactions_address_so_far_query = "SELECT COUNT(*) FROM txout TOUT JOIN txin TI ON TOUT.tx_id = TI.tx_id WHERE TOUT.tx_id < ? AND TOUT.address = ?"
used_so_far_query = "SELECT EXISTS(SELECT * FROM txout TOUT JOIN txin TI ON TOUT.tx_id = TI.tx_id WHERE TOUT.tx_id < ? AND TOUT.address = ?)"

max_txid_query = "SELECT MAX(tx_id) FROM tx"
max_block_query = "SELECT MAX(block_id) FROM blocks"
last_seen_query = "SELECT last_seen FROM addresses WHERE address = ?"
get_features_query = "SELECT * FROM addresses WHERE address = ?"

features_schema_prepend = """
PRAGMA page_size = 4096;
CREATE TABLE IF NOT EXISTS addresses(
address TEXT NOT NULL PRIMARY KEY,
first_seen INT,
last_seen INT,
recv INT,
sent INT,
balance INT,
n_tx INT,
"""

features_clusters_schema_prepend = """
CREATE TABLE IF NOT EXISTS clusters(
cluster_id INT NOT NULL PRIMARY KEY,
first_seen INT,
last_seen INT,
recv INT,
sent INT,
n_tx INT,
"""

features_update_partial_query = "UPDATE addresses SET BITCOINTALK_USER = ?, BITCOINOTC_USER = ?, SCAMMER = ?, SHAREHOLDER = ?, CASASCIUS = ?, FBI = ?, SILKROAD = ?, KILLER = ?, MALWARE = ?, cluster_id = ? WHERE address = ?"

def update_features(n, table):
	return "INSERT OR REPLACE INTO %s VALUES (" % table + '?,'*(n-1) + '?)'