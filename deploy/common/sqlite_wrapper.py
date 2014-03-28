import sqlite3

class SQLiteWrapper:
	def __init__(self, db):
		self.conn = sqlite3.connect(db)
		self.cursor = self.conn.cursor()
		self.cursor.execute("PRAGMA cache_size=4000")
		self.conn.commit()

	def query(self, sql, params=None, fetch_one=False, multi=False):
		try:
			with self.conn as conn:
				cursor = conn.cursor()
				if multi:
					cursor.executescript(sql)
				if params is None and not multi:
					cursor.execute(sql)
				if params is not None and not multi:
					cursor.execute(sql, params)
				if fetch_one:
					return cursor.fetchone()[0]
				if not multi:
					return cursor.fetchall()
		except Exception as e:
			raise Exception('Error in executing query ' + sql + ': ' + format(e))

	def close(self):
		self.conn.close()