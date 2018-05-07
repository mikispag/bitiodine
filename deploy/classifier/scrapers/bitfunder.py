#!/usr/bin/env python3

import urllib.request
import json
import csv
import os
from bitcoin_functions import isBTCAddress

urls = ['https://bitfunder.com/assetlist.json']

try:
	os.remove('../Lists/shareholders.csv')
except:
	pass

for url in urls:
	request = urllib.request.Request(url)
	request.add_header('User-Agent', 'Mozilla/5.0 (Windows NT 6.1; WOW64; rv:22.0) Gecko/20100101 Firefox/22.0')

	res = urllib.request.urlopen(request)
	assets = json.loads(res.readall().decode('utf-8'))
	duplicate_list = []

	print(len(assets), "assets retrieved.")

	with open('../Lists/shareholders.csv', 'a') as f:
		writer = csv.writer(f)

		for asset in assets:
			address = asset['user_btc_address']

			if address not in duplicate_list and isBTCAddress(address):
				writer.writerow([address, "1"])
				duplicate_list += address