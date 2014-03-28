#!/usr/bin/env python3
import networkx as nx
import pickle
from collections import defaultdict
from multiprocessing import Pool

from sys import argv

# Config
# Cluster number
cluster_n = int(argv[1])
dest_addr = argv[2]

def find(address):
	try:
		paths = list(nx.all_simple_paths(G, source=address))
		print("Added %d new paths from address %s with min length %d." %(len(paths), address, min([len(x) for x in paths])))
	except:
		paths = []

	if len(paths) > 0:
		return paths
	else:
		return None

with open("../clusterizer/clusters.dat", "rb") as cf:
	users = pickle.load(cf)

print("Clusters loaded.")

addresses = set()
for address, cluster in users.items():
	if cluster == cluster_n:
		addresses.add(address)
print("%d addresses loaded." % len(addresses))
del users

with open('../grapher/tx_graph.dat', "rb") as infile:
	G = pickle.load(infile)

print("Graph loaded.")

paths = []

p = Pool()

with open(str(cluster_n) + "_to_" + str(dest_addr) + ".txt", 'w') as f:

	res = p.map(find, addresses)

	for new_paths in res:
		if new_paths is not None:
			paths += new_paths

	# Sort paths by length
	paths.sort(key=len)

	tx_hashes = defaultdict(list)

	for i, path in enumerate(paths):
		for index in range(len(path) - 1):
			temp_hashes = []
			for index, value in G[path[index]][path[index+1]].items():
				temp_hashes.append(value['tx_hash'])
			tx_hashes[i].append(','.join(temp_hashes))

	for path in paths:
		f.write("%s\n" % '->'.join(path))

	f.write("\n\n")

	for path in tx_hashes.values():
		f.write("%s\n" % '->'.join(path))