#!/usr/bin/env python3

from bitcoin_functions import *
from sys import exit
import urllib.request, urllib.error
import csv, re, os

try:
	html = urllib.request.urlopen('http://blockchain.info/tags?filter=4').read().decode('utf-8')
except urllib.error.URLError as e:
	print(e.reason)
	exit(1)

results = re.findall(r"<span class=\"tag\" id=\"(\b1[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]{20,40}\b)\">([^\"]+)</span>", html)

with open('../Lists/bitcoinotc.csv', 'a') as f:
	writer = csv.writer(f)

	for result in results:
		address, username = result
		print("Adding user %s with address %s..." % (username, address))

		if not isBTCAddress(address):
			continue

		writer.writerow([address, username])

os.system("cp ../Lists/bitcoinotc.csv /tmp/temp-bitcoinotc.csv")
os.system("cat /tmp/temp-bitcoin-otc.csv | sort | uniq > ../Lists/bitcoinotc.csv")
os.system("rm -f /tmp/temp-bitcoin-otc.csv")