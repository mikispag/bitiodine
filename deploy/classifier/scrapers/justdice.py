#!/usr/bin/env python3

import urllib.request
import json
import csv
import os
import re
import sys
from time import sleep

url = 'https://just-dice.com/roll/'

try:
	with open('../Lists/justdice.csv') as f:
		reader = csv.reader(f)
		min_roll = max(int(row[3]) for row in reader) + 1
except:
	min_roll = 149527380

for roll in range(min_roll, 151373900):
	sleep(5) # Throttled
	request = urllib.request.Request(url + str(roll))
	request.add_header('User-Agent', 'Mozilla/5.0 (Windows NT 6.1; WOW64; rv:22.0) Gecko/20100101 Firefox/22.0')

	while True:
		try:
			html = urllib.request.urlopen(request).readall().decode('utf-8')

			regex = re.compile("moment\('(\d+)'", re.IGNORECASE)
			r = regex.search(html)
			timestamp = int(r.group(1))

			regex = re.compile("<div class=\"slabel\">profit</div><span> (\S+)", re.IGNORECASE)
			r = regex.search(html)
			value = float(r.group(1))

			regex = re.compile("<div class=\"slabel\">payout multiplier</div><span> ([\d\.]+)", re.IGNORECASE)
			r = regex.search(html)
			multiplier = float(r.group(1))
		except:
			print("Error with roll", roll)
			sleep(60)
			continue
		break

	print(timestamp)

	with open('../Lists/justdice.csv', 'a') as f:
		writer = csv.writer(f)
		writer.writerow([timestamp, "%.8f" % value, "%.2f" % (100.0/multiplier), roll])
		f.flush()