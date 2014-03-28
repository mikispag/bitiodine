#!/usr/bin/env python3

from sys import argv

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *
from collections import Counter
from pprint import pprint
from datetime import datetime

import argparse
import itertools
import numpy
import csv
###

FILENAME = "clusters_heur1"
FIX_TIME = 1329523200

parser = argparse.ArgumentParser(description="BitIodine Clusterizer: groups addresses in ownership clusters.")
parser.add_argument('-d', dest='db', default="../blockchain/blockchain.sqlite",
				   help='SQLite database path')
parser.add_argument("--generate-clusters", action="store_true", dest="generate", default=False,
	help="Generate clusters (takes a long time)")
parser.add_argument("--load-clusters", action="store_true", dest="load", default=False,
	help="Load a previously generated clusters from disk")
options = parser.parse_args()

db = SQLiteWrapper(options.db)

if options.generate:
	try:
		max_txid_res = db.query(max_txid_query, fetch_one=True)
	except Exception as e:
		die(e)

	users, loaded = {}, False
	try:
		users, min_txid = load(FILENAME)
		loaded = True
	except:
		min_txid = 1

	try:
		# Retrieve maximum cluster ID
		max_cluster_id = max(users.values())
	except ValueError:
		# users is empty
		max_cluster_id = 0

	print("Scanning %d transactions, starting from %d." %(max_txid_res, min_txid))

	for tx_id in range(min_txid, max_txid_res + 1):
		# Save progress to files
		if tx_id % 1000000 == 0 and not loaded:
			print("TRANSACTION ID: %d" % (tx_id))
			save(users, FILENAME, tx_id)

		loaded = False

		try:
			in_res = db.query(in_query_addr, (tx_id,))
			out_res = db.query(out_query_addr, (tx_id,))
		except Exception as e:
			print(e)
			continue

		# IN - Heuristic 1 - multi-input transactions
		found = None
		for line in in_res:
			address = line[0]
			if address is None:
				continue
			pos = users.get(address)
			if pos is not None:
				users[address] = pos
				found = pos
			break
		else:
			continue

		if found is None:
			max_cluster_id += 1
			found = max_cluster_id

		for address in in_res:
			users[address[0]] = found

	users = save(users, FILENAME, max_txid_res)

if options.load:
	try:
		users, _ = load(FILENAME)
		print("Clusters loaded - %d clusters, %d addresses in clusters." % (len(set(users.values())), len(users)))
	except Exception as e:
		die(e)

	counter = Counter(users.values())
	top10 = counter.most_common(10)

	print("Top clusters:")
	print("Cluster ID\t\t\tSize")
	for candidate, size in top10:
		print("%d\t\t\t\t%d" % (candidate, size))
	print()
	lengths = list(counter.values())

	users_no_singletons = stripSingletons(users)

	counter2 = Counter(users_no_singletons.values())
	lengths2 = list(counter2.values())

	print("Minimum cluster size:", min(lengths))
	print("Maximum cluster size:", max(lengths))
	print("Average:", numpy.mean(lengths))
	print("Median:", numpy.median(lengths))
	print("Addresses clustered (no singletons):", len(users_no_singletons))

	print("No Singletons:")

	print("Clusters:", len(set(users_no_singletons.values())))
	print("Minimum cluster size:", min(lengths2))
	print("Maximum cluster size:", max(lengths2))
	print("Average:", numpy.mean(lengths2))
	print("Median:", numpy.median(lengths2))

	# Generate histogram
	hist, bin_edges = numpy.histogram(lengths, bins=max(lengths)-1)

	with open("clusters_histogram_heur1.csv", "w") as f:
		writer = csv.writer(f)
		writer.writerow(['size', 'count'])
		for i in range(0, len(hist)):
			writer.writerow([int(bin_edges[i]), hist[i]])
