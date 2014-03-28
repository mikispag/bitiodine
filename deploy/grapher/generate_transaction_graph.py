#!/usr/bin/env python3
import networkx as nx

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *
from collections import Counter

###

FILENAME = "tx_graph"
db = SQLiteWrapper('../blockchain/blockchain.sqlite')

try:
  max_txid_res = db.query(max_txid_query, fetch_one=True)
except Exception as e:
  die(e)

G = nx.MultiDiGraph()
min_txid = 1

try:
  G, min_txid = load(FILENAME)
except Exception as e:
  print(e)

print("Scanning %d transactions, starting from %d." %(max_txid_res, min_txid))

for tx_id in range(min_txid, max_txid_res + 1):

  # Save progress to files
  if tx_id % 1000000 == 0:
    print("TRANSACTION ID: %d" % (tx_id))
    save(G, FILENAME, tx_id)
    print("%d nodes, %d edges so far." % (nx.number_of_nodes(G),nx.number_of_edges(G)))

  try:
    in_res = db.query(in_query_addr, (tx_id,))
    out_res = db.query(out_query_addr, (tx_id,))
    tx_hash = db.query(tx_hash_query, (tx_id,), fetch_one=True)
  except Exception as e:
    print(e)
    # Just go to the next transaction
    continue

  # IN
  in_addr = set()
  out_addr = set()
  for line in in_res:
    address = line[0]
    if address is not None:
    	in_addr.add(address)
    else:
      in_addr.add("GENERATED")

  # OUT  
  for out in out_res:
    if out[0] not in in_addr:
      out_addr.add(out[0])

  for in_address in in_addr:
    G.add_node(in_address)

  for out_address in out_addr:
    G.add_node(out_address)

  for in_address in in_addr:
    for out_address in out_addr:
      G.add_edge(in_address, out_address, tx_hash=tx_hash)

save(G, FILENAME, tx_id)