#!/usr/bin/env python3

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *
from collections import Counter

###

def padWithSpaces(address):
  if len(address) < 34:
    address += " " * (34 - len(address))
  return address

FILENAME = "tx_graph.lgf.new"
db = SQLiteWrapper('../blockchain/blockchain.sqlite')

min_txid_res = 0

try:
  addresses_res = db.query("SELECT DISTINCT address FROM txout")
except Exception as e:
  die(e)

with open(FILENAME, 'w') as f:

  f.write("@nodes\n")
  f.write("label\n")

  for address in addresses_res:
    f.write(address[0] + "\n")

  f.write("\n")
  f.write("@arcs\n")
  f.write(" " * 35)
  f.write(" " * 35)
  f.write("tx_hash\n")

  if os.path.isfile("tx_graph.lgf"):
    with open("tx_graph.lgf") as f_r:
      resume = False
      for line in f_r:
        if resume and line == "\n":
          break
        if resume:
          tx_hash = line
          f.write(line)
        elif "tx_hash\n" in line:
          resume = True
    try:
      min_txid_res = db.query(txhash_to_txid_query, (tx_hash.split()[2].rstrip(),), fetch_one=True)
    except Exception as e:
      die(e)

  try:
    max_txid_res = db.query(max_txid_query, fetch_one=True)
  except Exception as e:
    die(e)

  print("Scanning %d transactions." % (max_txid_res - min_txid_res))

  for tx_id in range(min_txid_res + 1, max_txid_res + 1):
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
      for out_address in out_addr:
        f.write(padWithSpaces(in_address) + " " + padWithSpaces(out_address) + " " + tx_hash + "\n")

  f.write("\n")
  f.write("@attributes\n")
  f.write("source 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa\n") # Genesis address

os.rename("tx_graph.lgf.new", "tx_graph.lgf");