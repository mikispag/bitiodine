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

###
db = SQLiteWrapper('../blockchain/blockchain.sqlite')

parser = argparse.ArgumentParser(description='Generate user graph based on transactions on a time interval desired')
parser.add_argument("--min-time", dest="min_time")
parser.add_argument("--max-time", dest="max_time")
parser.add_argument("--out-filename", dest="output_filename")
args = parser.parse_args()

# Load clusters
with open("../clusterizer/clusters.dat", "rb") as infile:
	users = pickle.load(infile)

print("Clusters loaded - %d addresses." % len(users))

users = stripSingletons(users)

print("Singletons stripped - %d addresses." % len(users))

try:
  amount_txids = db.query(number_of_transactions_between_time_interval, (args.min_time, args.max_time,))[0][0]
  min_tx_id, max_tx_id = db.query(max_min_transaction_ids_time_interval, (args.min_time, args.max_time,))[0]
 
except Exception as e:
  die(e)

G = nx.DiGraph()
min_txid = 1

try:
  G, min_tx_id = load(args.output_filename)
except:
  pass

print("Scanning %d transactions, starting from %d." %(amount_txids, min_tx_id))

for tx_id in range(min_tx_id, max_tx_id+1):

  source_is_addr, dest_is_addr = False, False

  # Save progress to files
  if math.fabs(tx_id - 1000000) % 1000000 == 0:
    print("TRANSACTION ID: %d" % (tx_id))
    save(G, args.output_filename, tx_id)
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

save(G, args.output_filename, tx_id)
