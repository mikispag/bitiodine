from sys import exit
from shutil import copyfile
from os import remove as deletefile
from os.path import isfile

import pickle

def die(msg=''):
	print(msg)
	exit(1)

def save(obj, filename, tx_id):
	try:
		if isfile("%s.dat" % filename):
			copyfile("%s.dat" % filename, "%s.dat.bak" % filename)
		with open("%s.dat" % filename, "wb") as outfile:
			pickle.dump(obj, outfile, 2)
		if isfile("%s.dat.bak" % filename):
			deletefile("%s.dat.bak" % filename)
	except Exception as e:
		print(e)
		return

	with open("%s_progress.dat" % filename, "w") as f:
		f.write(str(tx_id))

def load(filename):
	with open("%s.dat" % filename, "rb") as infile:
		obj = pickle.load(infile)
	with open("%s_progress.dat" % filename) as f:
		min_txid = int(f.readline())

	return obj, min_txid

def stripSingletons(users):
	# Filter Users (strip singletons)
	seen, repeated = set(), set()

	for cluster in users.values():
		if cluster in seen:
			repeated.add(cluster)
		else:
			seen.add(cluster)

	return dict((k, v) for (k, v) in users.items() if v in repeated)