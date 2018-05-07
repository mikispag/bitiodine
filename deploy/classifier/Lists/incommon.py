#!/usr/bin/env python3

import csv

addresses = set()

with open('bitcointalk.csv') as bt:
	reader = csv.reader(bt)
	for line in reader:
		address = line[0]
		addresses.add(address)

with open('bitcoinotc.csv') as bo:
	reader = csv.reader(bo)
	for line in reader:
		address = line[0]
		if address in addresses:
			print(address)