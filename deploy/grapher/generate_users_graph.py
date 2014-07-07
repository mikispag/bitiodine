#!/usr/bin/env python3
import networkx as nx

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *

###
FILENAME = "users_graph"
db = SQLiteWrapper('../blockchain/blockchain.sqlite')

# Load clusters
with open("../clusterizer/clusters.dat", "rb") as infile:
	users = pickle.load(infile)

print("Clusters loaded - %d addresses." % len(users))

users = stripSingletons(users)

print("Singletons stripped - %d addresses." % len(users))

try:
  max_txid_res = db.query(max_txid_query, fetch_one=True)
except Exception as e:
  die(e)

G = nx.DiGraph()
min_txid = 1

try:
  G, min_txid = load(FILENAME)
except:
  pass

print("Scanning %d transactions, starting from %d." %(max_txid_res, min_txid))

for tx_id in range(min_txid, max_txid_res + 1):

  source_is_addr, dest_is_addr = False, False

  # Save progress to files
  if tx_id % 1000000 == 0:
    print("TRANSACTION ID: %d" % (tx_id))
    save(G, FILENAME, tx_id)
    print(nx.number_of_nodes(G), "nodes,", nx.number_of_edges(G), "edges so far.")

  try:
    in_res = db.query(in_query_addr, (tx_id,))
    out_res = db.query(out_query_addr, (tx_id,))
    tx_hash = db.query(tx_hash_query, (tx_id,), fetch_one=True)
  except:
    # Just go to the next transaction
    continue

  in_addr, out_addr = set(), set()
  source = None
  # IN
  for line in in_res:
    address = line[0]
    if address is None:
      continue
    pos = users.get(address)
    if pos is not None:
      source = pos
      break
  else:
    continue

  if source is None:
    in_addr = set([line[0] for line in in_res])
    source_is_addr = True
  else:
    in_addr.add(str(source))

  # OUT
  for out in out_res:
    out_address = out[0]
    if out_address is not None:
      dest = users.get(out_address)

      if dest is None:
        dest = out_address
        dest_is_addr = True

      out_addr.add(out_address)

  for in_address in in_addr:
    G.add_node(in_address)

  for out_address in out_addr:
    G.add_node(out_address)

  for in_address in in_addr:
    for out_address in out_addr:
      G.add_edge(in_address, out_address, tx_hash=tx_hash)

save(G, FILENAME, tx_id)
