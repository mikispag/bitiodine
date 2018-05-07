#!/usr/bin/env python3
import networkx as nx
import pickle
from collections import defaultdict
from sys import argv

# Config
src_addr = argv[1]

with open('../grapher/tx_graph.dat', "rb") as infile:
	G = pickle.load(infile)

print("Graph loaded.")

with open(str(src_addr) + "_to_" + str(dest_addr) + ".txt", 'w') as f:

	paths = list(nx.all_simple_paths(G, source=src_addr))

	print("Added %d new paths from address %s with min length %d." %(len(paths), src_addr, min([len(x) for x in paths])))

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