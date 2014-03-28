#!/usr/bin/env python3

import os, sys
lib_path = os.path.abspath('../common')
sys.path.append(lib_path)

from functions import *
from features import Features
from sqlite_wrapper import SQLiteWrapper
from pprint import pprint
from datetime import datetime
from queries import *
from util import *
from collections import defaultdict
import pickle
import argparse
import logging

logging.basicConfig(level=logging.DEBUG)

parser = argparse.ArgumentParser(description="BitIodine Classifier")
parser.add_argument('-d', dest='db', default="features.sqlite",
				   help='SQLite database path')
parser.add_argument("-a", dest="address", default=None, help="Classify a single address.")
parser.add_argument("-af", dest="address_filename", default=None, help="Classify every address in a text file, one per line.")
parser.add_argument("-cf", dest="cluster_filename", default=None, help="Classify every cluster in a text file, one per line.")
parser.add_argument("-c", dest="cluster", type=int, default=None, help="Classify a single cluster.")
parser.add_argument("--all-clusters", action="store_true", dest="all_clusters", default=False, help="Classify every cluster.")
options = parser.parse_args()

db = SQLiteWrapper(options.db)

try:
	db_blockchain = SQLiteWrapper("../blockchain/blockchain.sqlite")
	max_block = int(db_blockchain.query(max_block_query, fetch_one=True))
	db_blockchain.close()
except:
	max_block = 0

f = Features()

scores = f.features
labels = f.labels
labels_string = f.labels_string

with open("../grapher/tx_graph.dat", "rb") as gf:
	G = pickle.load(gf)
print("Graph loaded.")

with open("../clusterizer/clusters.dat", "rb") as cf:
	users = pickle.load(cf)
print("Clusters loaded.")

users = stripSingletons(users)

print("Singletons stripped.")

clusters = None

if options.cluster_filename is not None:
	with open(options.cluster_filename, "r") as af:
		clusters = [int(line.strip()) for line in af]
	options.cluster = -1
elif options.address_filename is not None:
	with open(options.address_filename, "r") as af:
		addresses = [line.strip() for line in af]
elif options.address is not None:
	addresses = [options.address]
if options.cluster is not None:
	if options.cluster != -1:
		print("Classifying cluster %d..." % options.cluster)
		clusters = [options.cluster]
	else:
		print("Classifying clusters...")
	addresses = []
	for address, cluster in users.items():
		if cluster in clusters:
			addresses.append(address)
	if len(addresses) == 0:
		die("Cluster is empty, singleton or not existent.")
	print("Classifying %d address in the cluster." % len(addresses))
elif options.all_clusters:
	print("Classifying all clusters...")
	addresses = users.keys()
else:
	addresses = G

features_schema = features_schema_prepend
features_schema_clusters = features_clusters_schema_prepend

for score in scores:
	features_schema += "%s REAL,\n" % score
	features_schema_clusters += "%s REAL,\n" % score
for label in labels:
	features_schema += "%s BOOLEAN,\n" % label
	features_schema_clusters += "%s REAL,\n" % label
for label in labels_string:
	features_schema += "%s TEXT,\n" % label
	features_schema_clusters += "%s TEXT,\n" % label

features_schema += """cluster_id INT);
CREATE INDEX IF NOT EXISTS x_cluster_id ON addresses (cluster_id);
CREATE INDEX IF NOT EXISTS x_last_seen ON addresses (last_seen);
"""

features_schema_clusters += """
min_balance INT,
max_balance INT,
avg_balance INT);
"""

db.query(features_schema, multi=True)
db.query(features_schema_clusters, multi=True)

features_update_query = update_features(8 + len(scores) + len(labels) + len(labels_string), table='addresses')
features_update_cluster_query = update_features(9 + len(scores) + len(labels) + len(labels_string), table='clusters')

# Parameters
DELTA_OLD = 6*30*24*60*60
DELTA_NEW = 30*24*60*60
DELTA_RECENT = 7*24*60*60

features_all = defaultdict(list)

for address in addresses:
	print("Classifying address %s..." % address)
	try:
		first_seen, last_seen, recv, sent, balance, n_tx, in_addresses, out_addresses = getAddressInfo(address, G, db, max_block, verbose=True)
	except Exception as e:
		logging.debug("Exception in getAddressInfo()!")
		logging.exception(e)
		continue

	features = {}
	cluster_id = users.get(address)

	features['first_seen'] = first_seen
	features['last_seen'] = last_seen
	features['recv'] = recv
	features['sent'] = sent
	features['balance'] = balance
	features['n_tx'] = n_tx

	if first_seen is None and in_addresses is None and out_addresses is None:
		features['BITCOINTALK_USER'] = f.queryCSV('bitcointalk', address)
		features['BITCOINOTC_USER'] = f.queryCSV('bitcoinotc', address)
		features['SCAMMER'] = f.isInList(address, 'scammers')
		features['SHAREHOLDER'] = f.isInList(address, 'shareholders')
		features['CASASCIUS'] = f.isInList(address, 'casascius')
		features['FBI'] = f.isInList(address, 'FBI')
		features['SILKROAD'] = f.isInList(address, 'silkroad')
		features['KILLER'] = f.isInList(address, 'killers')
		features['MALWARE'] = f.isInList(address, 'malware')

		try:
			db.query(features_update_partial_query, [features['BITCOINTALK_USER'], features['BITCOINOTC_USER'], features['SCAMMER'], features['SHAREHOLDER'], features['CASASCIUS'], features['FBI'], features['SILKROAD'], features['KILLER'], features['MALWARE'], cluster_id, address])
		except:
			pass

		if options.cluster is not None or options.all_clusters is not None:
			features = f.queryDB(db, address)
			features_all[cluster_id].append(features)

		continue

	sum_in, sum_out = sum(in_addresses.values()), sum(out_addresses.values())
	sum_all = sum_in + sum_out
	time_delta = last_seen - first_seen


	for feature in scores:
		features[feature] = 0
		feature_addresses = f.getFeature(feature)

		# Add generated coins to mining count
		# (mining is the first feature)
		if feature == 'mining' and in_addresses['GENERATED'] > 0:
			features['MINER'] = True
			features[feature] += in_addresses['GENERATED']
			del in_addresses['GENERATED']
		else:
			features['MINER'] = False

		# OUT
		if feature != 'mining':
			for out_address in out_addresses:
				if out_address in feature_addresses:
					features[feature] += out_addresses[out_address]

		# IN
		for in_address in in_addresses:
			if in_address in feature_addresses:
				features[feature] += in_addresses[in_address]

	for feature in scores:
		if feature == 'mining':
			if sum_in == 0:
				sum_in = 1
			features[feature] /= sum_in
		else:
			if sum_all == 0:
				sum_all = 1
			features[feature] /= sum_all

	# Add manual features
	current_utc_timestamp = int(datetime.utcnow().strftime("%s"))

	# Boolean labels
	features['OTA'] = n_tx == 1
	features['OLD'] = last_seen < current_utc_timestamp - DELTA_OLD
	features['NEW'] = first_seen > current_utc_timestamp - DELTA_NEW
	features['EMPTY'] = balance < 50000 # satoshis!
	features['EXHAUSTED'] = features['EMPTY'] and recv > balance
	features['RECENTLY_ACTIVE'] = last_seen > current_utc_timestamp - DELTA_RECENT
	features['ZOMBIE'] = features['EXHAUSTED'] and features['OLD']
	features['DISPOSABLE'] = features['OLD'] and n_tx < 5 and time_delta < 60
	features['SCAMMER'] = f.isInList(address, 'scammers')
	features['SHAREHOLDER'] = f.isInList(address, 'shareholders')
	features['CASASCIUS'] = f.isInList(address, 'casascius')
	features['FBI'] = f.isInList(address, 'FBI')
	features['SILKROAD'] = f.isInList(address, 'silkroad')
	features['KILLER'] = f.isInList(address, 'killers')
	features['MALWARE'] = f.isInList(address, 'malware')

	# String labels
	features['BITCOINTALK_USER'] = f.queryCSV('bitcointalk', address)
	features['BITCOINOTC_USER'] = f.queryCSV('bitcoinotc', address)

	if options.cluster is None and options.all_clusters is None:
		pprint(features)
	else:
		features_all[cluster_id].append(features)

	# Update features DB
	params = [address, first_seen, last_seen, recv, sent, balance, n_tx]

	for x in scores + labels + labels_string:
		params.append(features[x])

	# Cluster ID
	params.append(cluster_id)

	db.query(features_update_query, params)

if options.cluster is not None:
	for cluster_id, features_list in features_all.items():
		t = {}
		for features in features_list:
			for feature in features:
				if feature not in t:
					t[feature] = []
				t[feature].append(features[feature])

		for feature in scores + labels:
			n_features = len(t[feature]) if len(t[feature]) > 0 else 1
			t[feature] = sum(t[feature]) / n_features

			# Priority labels for clusters (1 if at least one address in the cluster has the label)
			if feature in ['SCAMMER', 'FBI', 'SILKROAD', 'KILLER']:
				if t[feature] > 0:
					t[feature] = 1

			if t[feature] < 1e-6:
				t[feature] = 0

		for feature in labels_string:
			res = [x for x in t[feature] if x is not None]
			if len(res) == 0:
				t[feature] = None
			else:
				t[feature] = " ".join(res)

		t['first_seen'] = min(t['first_seen'])
		t['last_seen'] = max(t['last_seen'])
		t['recv'] = sum(t['recv'])
		t['sent'] = sum(t['sent'])
		t['n_tx'] = sum(t['n_tx'])
		t['max_balance'], t['min_balance'], t['avg_balance'] = max(t['balance']), min(t['balance']), sum(t['balance']) / n_features
		del t['balance']
		t['cluster_id'] = cluster_id

		pprint(t)

		# Update features DB
		params = [cluster_id, t['first_seen'], t['last_seen'], t['recv'], t['sent'], t['n_tx']]

		for x in scores + labels + labels_string:
			params.append(t[x])

		params.extend([t['min_balance'], t['max_balance'], t['avg_balance']])

		db.query(features_update_cluster_query, params)
