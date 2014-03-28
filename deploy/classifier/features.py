import csv
import pprint
from queries import *

class Features:

	features = ['mining', 'gambling', 'exchanges', 'wallets', 'bitcointalk', 'bitcoinotc', 'freebies', 'donations']
	labels = ['OTA', 'OLD', 'NEW', 'EMPTY', 'EXHAUSTED', 'RECENTLY_ACTIVE', 'ZOMBIE', 'SCAMMER', 'DISPOSABLE', 'MINER', 'SHAREHOLDER', 'CASASCIUS', 'FBI', 'SILKROAD', 'KILLER', 'MALWARE']
	labels_string = ['BITCOINTALK_USER', 'BITCOINOTC_USER']

	_data = {}

	def __init__(self):
		for feature in self.features + ['scammers', 'shareholders', 'casascius', 'FBI', 'silkroad', 'killers', 'malware']:
			self._data[feature] = self.readFile(feature + '.csv')

	def readFile(self, filename):
		x = set()
		try:
			with open('Lists/' + filename) as f:
				reader = csv.reader(f, dialect='excel', quoting=csv.QUOTE_NONE)
				for row in reader:
					address = row[0]
					x.add(address)
		except:
			return set()
		return x

	def getFeature(self, feature, generator=False):
		try:
			if generator:
				return (x for x in self._data[feature])
			else:
				return [x for x in self._data[feature]]
		except KeyError:
			return []

	def isInList(self, address, label):
		return address in self._data[label]

	def queryCSV(self, feature, address):
		filename = 'Lists/' + feature + '.csv'
		try:
			with open(filename) as f:
				reader = csv.reader(f, dialect='excel', quoting=csv.QUOTE_NONE)
				for row in reader:
					cur_address = row[0]
					username = row[1]
					if cur_address == address:
						return username
		except:
			return None
		return None

	def queryDB(self, db, address):
		features = {}

		res = db.query(get_features_query, (address,))
		res = res[0]
		features['first_seen'], features['last_seen'], features['recv'], features['sent'], features['balance'], features['n_tx'] = res[1:7]
		index = 7

		for feature in self.features:
			features[feature] = res[index]
			index += 1

		for label in self.labels:
			features[label] = False if res[index] == 0 else True
			index += 1

		for label in self.labels_string:
			features[label] = None if res[index] == 0 else res[index]
			index += 1

		features['cluster_id'] = res[index]

		return features
