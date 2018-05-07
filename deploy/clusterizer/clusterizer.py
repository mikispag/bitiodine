#!/usr/bin/env python3

from sys import argv

import os
import sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from sqlite_wrapper import SQLiteWrapper
from queries import *
from util import *
from collections import Counter, defaultdict
from pprint import pprint
from datetime import datetime

import argparse
import csv
import numpy
import operator
###

FILENAME = "clusters"

parser = argparse.ArgumentParser(
    description="BitIodine Clusterizer: groups addresses in ownership clusters.")
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
parser.add_argument("--sqlite", action="store_true", dest="sqlite",
                    default=False, help="Export clusters to a clusters.sqlite SQLite database")
options = parser.parse_args()

db = SQLiteWrapper(options.db)

if options.generate:
    try:
        max_txid_res = db.query(max_txid_query, fetch_one=True)
    except Exception as e:
        die(e)

    users, loaded = {}, False

    # Keep a cache for efficient value -> keys querying
    users_cache = defaultdict(set)

    try:
        users, min_txid = load(FILENAME)
        # Build cache
        for address, cluster in users.items():
            users_cache[cluster].add(address)
        loaded = True
    except:
        min_txid = 1

    try:
        # Retrieve maximum cluster ID
        max_cluster_id = max(users.values())
    except ValueError:
        # users is empty
        max_cluster_id = 0

    print("Scanning {} transactions, starting from {}.".format(
        max_txid_res, min_txid))

    for tx_id in range(min_txid, max_txid_res + 1):
        # Save progress to files
        if tx_id % 2000000 == 0 and not loaded:
            print("TRANSACTION ID: {}".format(tx_id))
            save(users, FILENAME, tx_id)

        loaded = False

        try:
            in_res = db.query(in_query_addr, (tx_id,))
        except Exception as e:
            print(e)
            continue

        # IN - Heuristic 1 - multi-input transactions
        new_cluster_id = None
        for line in in_res:
            address = line[0]
            if address is None:
                continue
            current_cluster_id = users.get(address)
            if new_cluster_id is not None and current_cluster_id != new_cluster_id:
                for address in users_cache[current_cluster_id]:
                    users_cache[new_cluster_id].add(address)
                    users[address] = new_cluster_id
                users_cache[current_cluster_id] = set()
            if new_cluster_id is None and current_cluster_id is not None:
                new_cluster_id = current_cluster_id

        if new_cluster_id is None:
            max_cluster_id += 1
            new_cluster_id = max_cluster_id

        for line in in_res:
            address = line[0]
            if address is None:
                continue
            old_cluster = users.get(address)
            if old_cluster is not None:
                users_cache[old_cluster].discard(address)
            users_cache[new_cluster_id].add(address)
            users[address] = new_cluster_id

    users = save(users, FILENAME, max_txid_res)

if options.load or options.csv or options.sqlite:
    try:
        users, _ = load(FILENAME)
        print("Clusters loaded - {} clusters, {} addresses in clusters.".format(
            len(set(users.values())), len(users)), file=sys.stderr)
    except Exception as e:
        die(e)

    if options.csv:
        with open(FILENAME + ".csv.new", "w") as f:
            writer = csv.writer(f)
            writer.writerow(["address", "cluster"])
            for address, cluster in users.items():
                writer.writerow([address, cluster])
        os.rename(FILENAME + ".csv.new", FILENAME + ".csv")
        sys.exit(0)

    if options.sqlite:
        cluster_db = SQLiteWrapper(FILENAME + ".sqlite.new")
        try:
            cluster_db.query(clusters_schema)
			clusters = 0
			rows = []
            for address, cluster in users.items():
				rows.append((cluster, address))
				clusters += 1
				if clusters == 10000:
					print("Updated 10,000 records.", file=sys.stderr)
                	cluster_db.query(add_cluster_query, many_rows=rows)
					rows = []
					clusters = 0
			cluster_db.query(add_cluster_query, many_rows=rows)
			cluster_db.close()
			os.rename(FILENAME + ".sqlite.new", FILENAME + ".sqlite")
        except Exception as e:
            die(e)
        sys.exit(0)

    counter = Counter(users.values())
    top10 = counter.most_common(10)

    print("Top clusters:")
    print("Cluster ID\t\t\tSize")
    for candidate, size in top10:
        print("{}\t\t\t\t{}".format(candidate, size))
    print()
    lengths = list(counter.values())

    users_no_singletons = stripSingletons(users)

    print("Minimum cluster size:", min(lengths))
    print("Maximum cluster size:", max(lengths))
    print("Average:", numpy.mean(lengths))
    print("Median:", numpy.median(lengths))
    print("Addresses clustered (no singletons):", len(users_no_singletons))

    # Generate histogram
    hist, bin_edges = numpy.histogram(lengths, bins=max(lengths) - 1)

    with open("clusters_histogram.csv", "w") as f:
        writer = csv.writer(f)
        writer.writerow(['size', 'count'])
        for i in range(len(hist)):
            writer.writerow([int(bin_edges[i]), hist[i]])

if options.print_cluster is not None:
    try:
        users, _ = load(FILENAME)
    except Exception as e:
        die(e)

    for address, cluster in users.items():
        if cluster == int(options.print_cluster):
            print(address)

if options.print_address is not None:
    try:
        users, _ = load(FILENAME)
    except Exception as e:
        die(e)

    print(users[options.print_address])
