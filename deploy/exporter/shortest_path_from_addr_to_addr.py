#!/usr/bin/env python3
import networkx as nx
import pickle

from sys import argv

# Config
src_addr = argv[1]
dest_addr = argv[2]

with open('../grapher/tx_graph.dat', "rb") as infile:
	G = pickle.load(infile)

print("Graph loaded.")

with open(str(src_addr) + "_to_" + str(dest_addr) + ".txt", 'w') as f:

	path = nx.shortest_path(G, source=src_addr, target=dest_addr)

	tx_hashes = []

	for path in paths:
		f.write("%s\n" % '->'.join(path))

	f.write("\n\n")

	for path in tx_hashes.values():
		f.write("%s\n" % '->'.join(path))


	for index in range(len(path) - 1):
		temp_hashes = []
		for index, value in G[path[index]][path[index+1]].items():
			temp_hashes.append(value['tx_hash'])
		tx_hashes.append(','.join(temp_hashes))

	print("Found shortest path from address %s to address %s with length %d." % (src_addr, dest_addr, len(path)))

	f.write("%s\n" % '->'.join(path))

	f.write("\n\n")

	f.write("%s\n" % '->'.join(tx_hashes))