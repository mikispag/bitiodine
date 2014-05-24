<?hh

class RedisWrapper {
	public static function getRedis() {
		$redis = new Redis();
		$redis->pconnect('127.0.0.1');
		return $redis;
	}
}
