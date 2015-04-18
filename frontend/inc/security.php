<?hh

class Security {

	private static $API_WHITELIST_KEYS = ImmSet {};
	private static $API_DAILY_MAX_PER_IP = 10;
	private static $WEB_DAILY_MAX_PER_IP = 100;

	public static function throttle_ip_api(): void {
		if (isset($_GET['key']) && self::$API_WHITELIST_KEYS->contains($_GET['key'])) {
			return;
		}

		$ip = $_SERVER["REMOTE_ADDR"];
		$redis = RedisWrapper::getRedis();

		if ($redis->setnx("A_$ip", 0)) {
			$redis->setTimeout("A_$ip", 3600 * 24);
			$redis->incr("A_$ip");
		} else {
			$count = intval($redis->get("A_$ip"));
			if ($count >= self::$API_DAILY_MAX_PER_IP) {
				die(json_error(403, "Daily maximum API requests for IP $ip reached. Please contact api@bitiodine.net to get whitelisted."));
			}
			$redis->incr("A_$ip");
		}
	}

	public static function throttle_ip_web(): void {
		if (isset($_GET['key']) && self::$API_WHITELIST_KEYS->contains($_GET['key'])) {
			return;
		}
		
		$ip = $_SERVER["REMOTE_ADDR"];
		$redis = RedisWrapper::getRedis();

		if ($redis->setnx("W_$ip", 0)) {
			$redis->setTimeout("W_$ip", 3600 * 24);
			$redis->incr("W_$ip");
		} else {
			$count = intval($redis->get("W_$ip"));
			if ($count >= self::$WEB_DAILY_MAX_PER_IP) {
				die("Daily maximum web requests for IP $ip reached. Please consider using the API and contact api@bitiodine.net to get whitelisted.");
			}
			$redis->incr("W_$ip");
		}
	}

}
