<?hh

require("AddressValidator.php");
require("log.php");
require("redis.php");

class BitIodine {

	private static $host = "127.0.0.1";
	private static $port = 8888;

	private static $debug = FALSE;
	private static $MAX_REPLY_SIZE = 500000;	// Lines
	private static $HOURS_CACHE = 4;

	public static function A2A(string $from, string $to): (int, Vector<string>, Vector<string>) {
		$redis = RedisWrapper::getRedis();

		$response_array = new Vector();
		$lines = 1;
		$request = "A2A_$from:$to";

		if (empty($from) || empty($to)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if ($from == $to) {
			write_log(true, $request, "SAME_ADDRESS");
			throw new RuntimeException("Source and destination addresses are the same.");
		}

		if (self::$debug === FALSE && (!AddressValidator::isValid($from) || !AddressValidator::isValid($to))) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if (!is_null($cached)) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, $errno, $errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "SHORTEST_PATH_A2A $from $to\r\n");
			    $response = "";
			    while ($response != "END" && $response != "500 No path." && $lines < self::$MAX_REPLY_SIZE) {
			    	$response = trim(fgets($fp));
			    	$lines++;
			    	$stream_metadata = stream_get_meta_data($fp);
			    	$timed_out = $stream_metadata["timed_out"];
			    	if ($timed_out === TRUE) {
			    		fclose($fp);
			    		write_log(false, $request, "TIMEOUT");
			    		throw new RuntimeException("Timeout while receiving data from BitIodine servers.");
			    	}
			        $response_array[] = $response;
			    }
			    fclose($fp);
			    $redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
			}
		}

		if ($response_array[0] == "500 No path.") {
			write_log((!is_null($cached)), $request, "NO_PATH");
			throw new RuntimeException("There is no connection between the two addresses.", 404);
		}

		$distance = intval($response_array[1]);
		$address_path = new Vector(explode('>', $response_array[2]));
		$tx_path = new Vector(explode('>', $response_array[3]));

		write_log((!is_null($cached)), $request, "OK");
		return tuple($distance, $address_path, $tx_path);
	}

	public static function stats(): (int, int) {
		$redis = RedisWrapper::getRedis();

		$response_array = new Vector();
		$lines = 1;
		$request = "STATS";

		$cached = $redis->get($request);

		if (!is_null($cached)) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, $errno, $errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "STATS\r\n");
			    while ($response != "END" && $response != "500 No path." && $lines < self::$MAX_REPLY_SIZE) {
			    	$response = trim(fgets($fp));
			    	$lines++;
			    	$stream_metadata = stream_get_meta_data($fp);
			    	$timed_out = $stream_metadata["timed_out"];
			    	if ($timed_out === TRUE) {
			    		fclose($fp);
			    		write_log(false, $request, "TIMEOUT");
			    		throw new RuntimeException("Timeout while receiving data from BitIodine servers.");
			    	}
			        $response_array[] = $response;
			    }
			    fclose($fp);
			    if (intval($response_array[1]) > 0) {
			    	$redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
				}
			}
		}

		$nodes = intval($response_array[1]);
		$arcs = intval($response_array[2]);

		write_log((!is_null($cached)), $request, "OK");
		return tuple($nodes, $arcs);
	}

	public static function neighbors(string $address): Vector<string> {
		$redis = RedisWrapper::getRedis();

		$response_array = new Vector();
		$lines = 1;
		$request = "N_$address";

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if (self::$debug === FALSE && !AddressValidator::isValid($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if (!is_null($cached)) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, $errno, $errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "PRINT_NEIGHBORS $address\r\n");
			    while ($response != "END" && $response != "500 Address not present in any cluster." && $lines < self::$MAX_REPLY_SIZE) {
			    	$response = trim(fgets($fp));
			    	$lines++;
			    	$stream_metadata = stream_get_meta_data($fp);
			    	$timed_out = $stream_metadata["timed_out"];
			    	if ($timed_out === TRUE) {
			    		fclose($fp);
			    		write_log(false, $request, "TIMEOUT");
			    		throw new RuntimeException("Timeout while receiving data from BitIodine servers.");
			    	}
			        $response_array[] = $response;
			    }
			    fclose($fp);
			    $redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
			}
		}

		if ($response_array[0] == "500 Address not present in any cluster.") {
			write_log((!is_null($cached)), $request, "NO_CLUSTER");
			throw new RuntimeException("The address is not part of a cluster.");
		}

		$response_array->pop();
		$response_array->removeKey(0);
		write_log((!is_null($cached)), $request, "OK");
		return new Vector($response_array);
	}

	public static function predecessors(string $address): Vector<string> {
		$redis = RedisWrapper::getRedis();

		$response_array = new Vector();
		$lines = 1;
		$request = "P_$address";

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if (self::$debug === FALSE && !AddressValidator::isValid($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if (!is_null($cached)) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, $errno, $errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "PREDECESSORS $address\r\n");
			    while ($response != "END" && $response != "500 No predecessors." && $lines < self::$MAX_REPLY_SIZE) {
			    	$response = trim(fgets($fp));
			    	$lines++;
			    	$stream_metadata = stream_get_meta_data($fp);
			    	$timed_out = $stream_metadata["timed_out"];
			    	if ($timed_out === TRUE) {
			    		write_log(false, $request, "TIMEOUT");
			    		fclose($fp);
			    		throw new RuntimeException("Timeout while receiving data from BitIodine servers.");
			    	}
			        $response_array[] = $response;
			    }
			    fclose($fp);
			    $redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
			}
		}

		if ($response_array[0] == "500 No predecessors.") {
			write_log((!is_null($cached)), $request, "NO_PREDECESSORS");
			throw new RuntimeException("The address has no predecessors.");
		}

		$response_array->pop();
		$response_array->removeKey(0);
		write_log((!is_null($cached)), $request, "OK");
		return new Vector(explode(',', $response_array[0]));
	}

	public static function successors(string $address): Vector<string> {
		$redis = RedisWrapper::getRedis();

		$response_array = new Vector();
		$lines = 1;
		$request = "S_$address";

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if (self::$debug === FALSE && !AddressValidator::isValid($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if (!is_null($cached)) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, $errno, $errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "SUCCESSORS $address\r\n");
			    while ($response != "END" && $response != "500 No successors." && $lines < self::$MAX_REPLY_SIZE) {
			    	$response = trim(fgets($fp));
			    	$lines++;
			    	$stream_metadata = stream_get_meta_data($fp);
			    	$timed_out = $stream_metadata["timed_out"];
			    	if ($timed_out === TRUE) {
			    		fclose($fp);
			    		write_log(false, $request, "TIMEOUT");
			    		throw new RuntimeException("Timeout while receiving data from BitIodine servers.");
			    	}
			        $response_array[] = $response;
			    }
			    fclose($fp);
			    $redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
			}
		}

		if ($response_array[0] == "500 No successors.") {
			write_log((!is_null($cached)), $request, "NO_SUCCESSORS");
			throw new RuntimeException("The address has no successors.");
		}

		$response_array->pop();
		$response_array->removeKey(0);
		write_log((!is_null($cached)), $request, "OK");
		return new Vector(explode(',', $response_array[0]));
	}
}
