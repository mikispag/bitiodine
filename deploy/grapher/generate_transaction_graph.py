#!/usr/bin/env python3
import networkx as nx

import argparse
import math

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *
from collections import Counter

###

db = SQLiteWrapper('../blockchain/blockchain.sqlite')

parser = argparse.ArgumentParser(description='Generate transaction graph based on transactions on a time interval desired')
parser.add_argument("--min-time", dest="min_time")
parser.add_argument("--max-time", dest="max_time")
parser.add_argument("--out-filename", dest="output_filename")
args = parser.parse_args()

try:
  amount_txids = db.query(number_of_transactions_between_time_interval, (args.min_time, args.max_time,))[0][0]
  min_tx_id, max_tx_id = db.query(max_min_transaction_ids_time_interval, (args.min_time, args.max_time,))[0]
  max_txid_res = db.query(max_txid_query, fetch_one=True)
except Exception as e:
  die(e)

G = nx.MultiDiGraph()

try:
  G, min_tx_id = load(args.output_filename)
except Exception as e:
  print(e)

print("Scanning %d transactions, starting from %d." %(amount_txids, min_tx_id))

for tx_id in range(min_tx_id, max_tx_id+1):

  # Save progress to files
  if math.fabs(tx_id - 1000000) % 1000000 == 0:
    print("TRANSACTION ID: %d" % (tx_id))
    save(G, args.output_filename, tx_id)
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

save(G, args.output_filename, max_tx_id+1)
