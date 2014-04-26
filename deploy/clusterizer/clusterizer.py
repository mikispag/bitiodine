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

FILENAME = "clusters"
FIX_TIME = 1329523200

parser = argparse.ArgumentParser(description="BitIodine Clusterizer: groups addresses in ownership clusters.")
parser.add_argument('-d', dest='db', default="../blockchain/blockchain.sqlite",
				   help='SQLite database path')
parser.add_argument("--generate-clusters", action="store_true", dest="generate", default=False,
	help="Generate clusters (takes a long time)")
parser.add_argument("--load-clusters", action="store_true", dest="load", default=False,
	help="Load a previously generated clusters from disk")
parser.add_argument("--print-cluster", dest="print_cluster", default=None,
	help="Display all addresses belonging to a cluster")
parser.add_argument("--print-address", dest="print_address", default=None,
	help="Display the cluster ID to which an address belongs")
parser.add_argument("--csv", action="store_true", dest="csv", default=False,
	help="Export clusters to a clusters.csv file")
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

		# OUT - Heuristic 2 - shadow addresses
		# Exploit bitcoin client bug - "change never last output"
		# https://bitcointalk.org/index.php?topic=128042.msg1398752#msg1398752
		# https://bitcointalk.org/index.php?topic=136289.msg1451700#msg1451700
		# 
		# Fixed on Jan 30, 2013
		# https://github.com/bitcoin/bitcoin/commit/ac7b8ea0864e925b0f5cf487be9acdf4a5d0c487#diff-d7618bdc04db23aa74d6a5a4198c58fd
		if len(out_res) == 2:
			address1 = out_res[0][0]
			address2 = out_res[1][0]
			try:
				appeared1_res = db.query(used_so_far_query, (tx_id, address1), fetch_one=True)
				appeared2_res = db.query(used_so_far_query, (tx_id, address2), fetch_one=True)
				time_res = db.query(time_query, (tx_id,), fetch_one=True)
			except Exception as e:
				die(e)

			if appeared1_res == 0 and (time_res < FIX_TIME or appeared2_res == 1):
				# Address 1 is never used and appeared, likely a shadow address, add to previous group
				# Exploits bitcoin client bug in case time_res < FIX_TIME or
				# is deterministic in case appeared2_res == 1
				# 
				# Fixed on Jan 30, 2013 on Git repo
				# https://github.com/bitcoin/bitcoin/commit/ac7b8ea0864e925b0f5cf487be9acdf4a5d0c487#diff-d7618bdc04db23aa74d6a5a4198c58fd
				# 
				# Next release: 0.8.0 on 18 Feb 2013
				# 
				# so only applies to transactions happened before 18 Feb 2013 (UNIX TIMESTAMP - FIX_TIME: 1329523200)
				users[address1] = found
			elif appeared2_res == 0 and appeared1_res == 1:
				# This is deterministic - last address is actually a shadow address
				users[address2] = found

	users = save(users, FILENAME, max_txid_res)

if options.load or options.csv:
	try:
		users, _ = load(FILENAME)
		print("Clusters loaded - %d clusters, %d addresses in clusters." % (len(set(users.values())), len(users)))
	except Exception as e:
		die(e)

	if options.csv:
		with open("clusters.csv.new", "w") as f:
			writer = csv.writer(f)
			writer.writerow(["address", "cluster"])
			for address, cluster in users.items():
				writer.writerow([address, cluster])
		os.rename("clusters.csv.new", "clusters.csv");
		sys.exit(0)

	counter = Counter(users.values())
	top10 = counter.most_common(10)

	print("Top clusters:")
	print("Cluster ID\t\t\tSize")
	for candidate, size in top10:
		print("%d\t\t\t\t%d" % (candidate, size))
	print()
	lengths = list(counter.values())

	users_no_singletons = stripSingletons(users)

	print("Minimum cluster size:", min(lengths))
	print("Maximum cluster size:", max(lengths))
	print("Average:", numpy.mean(lengths))
	print("Median:", numpy.median(lengths))
	print("Addresses clustered (no singletons):", len(users_no_singletons))

	# Generate histogram
	hist, bin_edges = numpy.histogram(lengths, bins=max(lengths)-1)

	with open("clusters_histogram.csv", "w") as f:
		writer = csv.writer(f)
		writer.writerow(['size', 'count'])
		for i in range(0, len(hist)):
			writer.writerow([int(bin_edges[i]), hist[i]])

if options.print_cluster is not None:
	try:
		users, _ = load(FILENAME)
		#print("Clusters loaded - %d clusters, %d addresses in clusters." % (len(set(users.values())), len(users)))
	except Exception as e:
		die(e)

	for address, cluster in users.items():
		if cluster == int(options.print_cluster):
			print(address)

if options.print_address is not None:
	try:
		users, _ = load(FILENAME)
		#print("Clusters loaded - %d clusters, %d addresses in clusters." % (len(set(users.values())), len(users)))
	except Exception as e:
		die(e)

	print(users[options.print_address])