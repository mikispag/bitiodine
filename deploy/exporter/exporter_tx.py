#!/usr/bin/env python3
import networkx as nx
import os
import pickle

from sys import argv

try:
	f = argv[1]
except:
	f = '../grapher/tx_graph.dat'

# Config
#
# Filter? (DOT will be a subgraph)
filter_dot = True

# minimum out degree of a
threshold_out_degree_src = 1 # n
# minimum in degree of b
threshold_in_degree_dst = 1 # n

with open(f, "rb") as infile:
	G = pickle.load(infile)

print("Graph loaded.")

f, _ = os.path.splitext(f)

nodes, edges = set(), []

with open(f + ".dot", 'w') as f:
	f.write('digraph G {\n');

	for u, v, d in G.edges_iter(data=True):

		if filter_dot and (G.out_degree(u) < threshold_out_degree_src or G.in_degree(v) < threshold_in_degree_dst):
			continue
		nodes.add(u)
		nodes.add(v)
		edges.append((u, v, d))

	print("Filtering results: %d nodes and %d edges." % (len(nodes), len(edges)))
	print("Generating a DOT file...")

	nodes = sorted(list(nodes))

	for n in nodes:
		f.write('"%s";\n' % n)

	del G
	f.write('\n')

	for edge in edges:
		(u, v, d) = edge
		f.write('"%s" -> "%s" [tx_hash=%s];\n' % (u, v, d['tx_hash']))

	f.write('};\n')