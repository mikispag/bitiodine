#!/usr/bin/env python3
import networkx as nx
import pickle
from multiprocessing import Pool
from collections import defaultdict

from sys import argv

# Config
# Cluster number
cluster_n = int(argv[1])
dest_addr = argv[2]

def find(address):
	try:
		path = nx.shortest_path(G, source=address, target=dest_addr)
		print("Found shortest path from address %s to address %s with length %d." % (address, dest_addr, len(path)))
	except:
		path = []

	if len(path) > 0:
		return path
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

	for new_path in res:
		if new_path is not None:
			paths.append(new_path)

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