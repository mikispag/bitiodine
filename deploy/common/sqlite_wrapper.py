import sqlite3

class SQLiteWrapper:
	def __init__(self, db):
		self.conn = sqlite3.connect(db)
		self.cursor = self.conn.cursor()
		self.cursor.execute("PRAGMA journal_mode=MEMORY")
		self.cursor.execute("PRAGMA cache_size=-16000")
		self.cursor.execute("PRAGMA synchronous=NORMAL")
		self.conn.commit()

	def query(self, sql, params=None, iterator=False, fetch_one=False, multi=False, many_rows=None):
		try:
			with self.conn as conn:
				cursor = conn.cursor()
				if many_rows:
					cursor.executemany(sql, many_rows)
					return
				if multi:
					cursor.executescript(sql)
				if params is None and not multi:
					cursor.execute(sql)
				if params and not multi:
					cursor.execute(sql, params)
				if iterator:
					return cursor
				if fetch_one:
					return cursor.fetchone()[0]
				if not multi:
					return cursor.fetchall()
		except Exception as e:
			raise Exception('Error in executing query ' + sql + ': ' + format(e))

	def close(self):
		self.conn.close()
