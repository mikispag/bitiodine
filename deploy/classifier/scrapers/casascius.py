#!/usr/bin/env python3

import urllib.request
import json
import csv
import os
from bitcoin_functions import isBTCAddress

urls = ['http://casascius.uberbills.com/api/?status=active', 'http://casascius.uberbills.com/api/?status=opened']

try:
	os.remove('../Lists/casascius.csv')
except:
	pass

for url in urls:
	res = urllib.request.urlopen(url)
	coins = json.loads(res.readall().decode('utf-8'))

	print(len(coins), "coins retrieved.")

	with open('../Lists/casascius.csv', 'a') as f:
		writer = csv.writer(f)

		for coin in coins:
			coin_type = str(coin['type'])
			address = coin['address']

			if coin_type == 'o5':
				coin_type = '0.5'

			if coin_type == 'o1':
				coin_type = '0.1'

			if len(coin_type) > 0 and isBTCAddress(address):
				writer.writerow([address, coin_type])