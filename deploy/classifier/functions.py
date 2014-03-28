import urllib.request, urllib.error
import simplejson as json
from pprint import pprint
from collections import Counter
from queries import *
from time import sleep
import sys
import logging

def getAddressInfo(address, G, db, max_block, verbose=False):
	MAX_HTTP_REQUESTS = 50

	try:
		last_seen_old = int(db.query(last_seen_query, (address,), fetch_one=True))
	except Exception as e:
		last_seen_old = 0

	resp = None
	while resp is None:
		try:
			resp = urllib.request.urlopen("http://blockchain.info/rawaddr/%s?api_code=LK75FDss&show_adv=1" % address)
		except urllib.error.URLError as e:
			logging.exception(e)
			sleep(5)
			pass

	json_array = json.loads(resp.read())

	n_tx = json_array['n_tx']
	recv = json_array['total_received']
	sent = json_array['total_sent']
	balance = json_array['final_balance']
	last_seen = json_array['txs'][0]['time']
	first_seen = None

	if last_seen > last_seen_old:
		if verbose:
			if last_seen_old > 0:
				print("last_seen > last_seen_old, %d > %d" % (last_seen, last_seen_old))
			else:
				print("Address not in DB. Full update...")

		while first_seen is None:
			try:
				sleep(1)
				first_seen = int(urllib.request.urlopen('http://blockchain.info/q/addressfirstseen/%s?api_code=LK75FDss&show_adv=1' % address).read())
			except urllib.error.URLError as e:
				logging.exception(e)
				sleep(5)
				pass

		txs = json_array['txs']
		in_addresses, out_addresses = [], []
		offset = 0
		horizon = False

		# Get recent transactions from blockchain.info
		while not horizon and len(txs) > 0 and offset < 50*MAX_HTTP_REQUESTS:
			offset += 50
			
			for tx in txs:
				try:
					block = tx['block_height']
					inputs = tx['inputs']
					outputs = tx['out']
				except:
					logging.debug("Skipped malformed transaction.")
					continue

				if block <= max_block:
					if verbose:
						print("block <= max_block, %d <= %d" % (block, max_block))
					horizon = True
					break

				if len(inputs) == 0:
					in_addresses.append('GENERATED')

				for inp in inputs:
					try:
						inp_addr = inp['prev_out']['addr']
						if inp_addr != address:
							in_addresses.append(inp_addr)
					except KeyError:
						continue

				for outp in outputs:
					outp_addr = outp['addr']
					if outp_addr != address:
						out_addresses.append(outp_addr)

			if not horizon:
				if verbose:
					print("Retrieving transactions to reach horizon...")
				resp = None
				while resp is None:
					try:
						url = "http://blockchain.info/rawaddr/%s?offset=%d&api_code=LK75FDss&show_adv=1" % (address, offset)
						if verbose:
							print("%d transactions..." % offset)
						resp = urllib.request.urlopen(url)
					except urllib.error.URLError as e:
						logging.exception(e)
						sleep(5)
						pass

				json_array = json.loads(resp.read())
				txs = json_array['txs']

		if not horizon and len(txs) > 0 and verbose:
			sys.stderr.write("Horizon not reached, this means results may be inaccurate.\n")

		# Get from DB (Graph)
		if address in G:
			for successor in G.successors(address):
				out_addresses.extend([successor])

			for predecessor in G.predecessors(address):
				in_addresses.extend([predecessor])

		print("Done.")
			
		return first_seen, last_seen, recv, sent, balance, n_tx, Counter(in_addresses), Counter(out_addresses)
	else:
		if verbose:
			print("Address is already up to date. Partial update...")
		return None, None, None, None, None, None, None, None