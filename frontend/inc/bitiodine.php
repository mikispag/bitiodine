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

	private static $errstr = "";
	private static $errno = 0;

	public static function getLabels(): ImmMap<string, int> {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$labels_map = Map {};
		$request = "GET_LABELS";

		$cached = $redis->get($request);

		if ($cached) {
			return new ImmMap(unserialize($cached));
		} else {
			$db = new SQLite3('/home/miki/bitiodine/deploy/clusterizer/cluster_labels.sqlite');
			$stmt = $db->prepare('SELECT cluster_id, label FROM cluster_labels ORDER BY label');
			$result = $stmt->execute();

			while ($row = $result->fetchArray()) {
			    $labels_map[$row['label']] = $row['cluster_id'];
			}
			$redis->set($request, serialize(new ImmMap($labels_map)), 3600 * self::$HOURS_CACHE);
		}

		write_log(($cached !== false), $request, "OK");
		return new ImmMap($labels_map);
	}

	public static function shortest_path_A2A(string $from, string $to): (int, Vector<string>, Vector<string>) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "SHORTEST_PATH_A2A_$from:$to";

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

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
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

		if ($response_array->isEmpty() || $response_array[0] == "500 No path.") {
			write_log(($cached !== false), $request, "NO_PATH");
			throw new RuntimeException("There is no connection between the two addresses.", 404);
		}

		$distance = intval($response_array[1]);
		$address_path = new Vector(explode('>', $response_array[2]));
		$tx_path = new Vector(explode('>', $response_array[3]));

		write_log(($cached !== false), $request, "OK");
		return tuple($distance, $address_path, $tx_path);
	}

	public static function A2A(string $from, string $to, int $min_time = 0, int $max_time = 2147483647, float $min_value = floatval(0), float $max_value = INF): (Vector<string>, Vector<int>, Vector<float>) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
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

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "A2A $from $to\r\n");
			    $response = "";
			    while ($response != "END" && $response != "500 No transactions." && $lines < self::$MAX_REPLY_SIZE) {
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

		if ($response_array->isEmpty() || $response_array[0] == "500 No transactions.") {
			write_log(($cached !== false), $request, "NO_TXS");
			throw new RuntimeException("There are no transactions between the two addresses.", 404);
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		$tx_hashes = Vector {};
		$timestamps = Vector {};
		$values = Vector {};
		foreach ($response_array as $line) {
			$parsed_line = new Vector(explode(',', $line));
			$timestamp = intval($parsed_line[1]);
			$value = floatval(intval($parsed_line[2])/10e8);
			if ($timestamp >= $min_time && $timestamp <= $max_time && $value >= $min_value && $value <= $max_value) {
				$tx_hashes[] = $parsed_line[0];
				$timestamps[] = $timestamp;
				$values[] = $value;
			}
		}

		write_log(($cached !== false), $request, "OK");
		return tuple($tx_hashes, $timestamps, $values);
	}

	public static function A2C(string $from, int $to_cluster, int $min_time = 0, int $max_time = 2147483647, float $min_value = floatval(0), float $max_value = INF): (Vector<string>, Vector<int>, Vector<float>) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "A2C_$from:$to_cluster";

		if (empty($from)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid address.");
		}

		if (empty($to_cluster) || !is_numeric($to_cluster)) {
			write_log(true, $request, "INVALID_CLUSTER");
			throw new RuntimeException("Invalid cluster.");
		}

		if (self::$debug === FALSE && (!AddressValidator::isValid($from))) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "A2C $from $to_cluster\r\n");
			    $response = "";
			    while ($response != "END" && $response != "500 No transactions." && $lines < self::$MAX_REPLY_SIZE) {
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

		if ($response_array->isEmpty() || $response_array[0] == "500 No transactions.") {
			write_log(($cached !== false), $request, "NO_TXS");
			throw new RuntimeException("There are no transactions between the address and the cluster.", 404);
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		$tx_hashes = Vector {};
		$timestamps = Vector {};
		$values = Vector {};
		foreach ($response_array as $line) {
			$parsed_line = new Vector(explode(',', $line));
			$timestamp = intval($parsed_line[1]);
			$value = floatval(intval($parsed_line[2])/10e8);
			if ($timestamp >= $min_time && $timestamp <= $max_time && $value >= $min_value && $value <= $max_value) {
				$tx_hashes[] = $parsed_line[0];
				$timestamps[] = $timestamp;
				$values[] = $value;
			}
		}

		write_log(($cached !== false), $request, "OK");
		return tuple($tx_hashes, $timestamps, $values);
	}

	public static function C2A(int $from_cluster, string $address, int $min_time = 0, int $max_time = 2147483647, float $min_value = floatval(0), float $max_value = INF): (Vector<string>, Vector<int>, Vector<float>) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "A2C_$from:$to_cluster";

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid address.");
		}

		if (empty($from_cluster) || !is_numeric($from_cluster)) {
			write_log(true, $request, "INVALID_CLUSTER");
			throw new RuntimeException("Invalid cluster.");
		}

		if (self::$debug === FALSE && (!AddressValidator::isValid($address))) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "C2A $from_cluster $address\r\n");
			    $response = "";
			    while ($response != "END" && $response != "500 No transactions." && $lines < self::$MAX_REPLY_SIZE) {
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

		if ($response_array->isEmpty() || $response_array[0] == "500 No transactions.") {
			write_log(($cached !== false), $request, "NO_TXS");
			throw new RuntimeException("There are no transactions between the cluster and the address.", 404);
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		$tx_hashes = Vector {};
		$timestamps = Vector {};
		$values = Vector {};
		foreach ($response_array as $line) {
			$parsed_line = new Vector(explode(',', $line));
			$timestamp = intval($parsed_line[1]);
			$value = floatval(intval($parsed_line[2])/10e8);
			if ($timestamp >= $min_time && $timestamp <= $max_time && $value >= $min_value && $value <= $max_value) {
				$tx_hashes[] = $parsed_line[0];
				$timestamps[] = $timestamp;
				$values[] = $value;
			}
		}

		write_log(($cached !== false), $request, "OK");
		return tuple($tx_hashes, $timestamps, $values);
	}

	public static function C2C(int $from_cluster, int $to_cluster, int $min_time = 0, int $max_time = 2147483647, float $min_value = floatval(0), float $max_value = INF): (Vector<string>, Vector<int>, Vector<float>) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "C2C_$from_cluster:$to_cluster";

		if (empty($from_cluster) || !is_numeric($from_cluster)) {
			write_log(true, $request, "INVALID_CLUSTER");
			throw new RuntimeException("Invalid cluster.");
		}

		if (empty($to_cluster) || !is_numeric($to_cluster)) {
			write_log(true, $request, "INVALID_CLUSTER");
			throw new RuntimeException("Invalid cluster.");
		}

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "C2C $from_cluster $to_cluster\r\n");
			    $response = "";
			    while ($response != "END" && $response != "500 No transactions." && $lines < self::$MAX_REPLY_SIZE) {
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

		if ($response_array->isEmpty() || $response_array[0] == "500 No transactions.") {
			write_log(($cached !== false), $request, "NO_TXS");
			throw new RuntimeException("There are no transactions between the two clusters.", 404);
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		$tx_hashes = Vector {};
		$timestamps = Vector {};
		$values = Vector {};
		foreach ($response_array as $line) {
			$parsed_line = new Vector(explode(',', $line));
			$timestamp = intval($parsed_line[1]);
			$value = floatval(intval($parsed_line[2])/10e8);
			if ($timestamp >= $min_time && $timestamp <= $max_time && $value >= $min_value && $value <= $max_value) {
				$tx_hashes[] = $parsed_line[0];
				$timestamps[] = $timestamp;
				$values[] = $value;
			}
		}

		write_log(($cached !== false), $request, "OK");
		return tuple($tx_hashes, $timestamps, $values);
	}

	public static function stats(): (int, int) {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "STATS";

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
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
			    if ($response_array->count() > 1 && intval($response_array[1]) > 0) {
			    	$redis->set($request, serialize($response_array), 3600 * self::$HOURS_CACHE);
				}
			}
		}

		$nodes = intval($response_array[1]);
		$arcs = intval($response_array[2]);

		write_log(($cached !== false), $request, "OK");
		return tuple($nodes, $arcs);
	}

	public static function cluster_id(string $address): int {
		$response = "";

		$request = "GC_$address";
		$response_array = Vector {};
		$lines = 1;

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if (self::$debug === FALSE && !AddressValidator::isValid($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
		if (!$fp) {
			write_log(false, $request, "SERVER_KO");
		    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
		} else {
			stream_set_timeout($fp, 30);
			// Consume welcome message
			fgets($fp);
			usleep(500000);
		    fwrite($fp, "PRINT_CLUSTER_ID $address\r\n");
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
		}

		if ($response_array->isEmpty() || $response_array[0] == "500 Address not present in any cluster.") {
			write_log(($cached !== false), $request, "NO_CLUSTER");
			throw new RuntimeException("The address is not part of a cluster.");
		}

		write_log(($cached !== false), $request, "OK");
		return intval($response_array[1]);
	}

	public static function print_cluster(int $cluster): Vector<string> {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
		$lines = 1;
		$request = "C_$cluster";

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
			if (!$fp) {
				write_log(false, $request, "SERVER_KO");
			    throw new RuntimeException("BitIodine servers are updating the blockchain and will be back soon.");
			} else {
				stream_set_timeout($fp, 30);
				// Consume welcome message
				fgets($fp);
				usleep(500000);
			    fwrite($fp, "PRINT_CLUSTER $cluster\r\n");
			    while ($response != "END" && $lines < self::$MAX_REPLY_SIZE) {
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

		if ($response_array->count() < 3) {
			write_log(($cached !== false), $request, "NO_CLUSTER");
			throw new RuntimeException("The cluster ID does not exist.");
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		write_log(($cached !== false), $request, "OK");
		return new Vector($response_array);
	}

	public static function neighbors(string $address): Vector<string> {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$cluster_id = self::cluster_id($address);

		$response_array = Vector {};
		$lines = 1;
		$request = "C_$cluster_id";

		if (empty($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		if (self::$debug === FALSE && !AddressValidator::isValid($address)) {
			write_log(true, $request, "INVALID_ADDRESS");
			throw new RuntimeException("Invalid addresses.");
		}

		$cached = $redis->get($request);

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
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

		if ($response_array->isEmpty() ||$response_array[0] == "500 Address not present in any cluster.") {
			write_log(($cached !== false), $request, "NO_CLUSTER");
			throw new RuntimeException("The address is not part of a cluster.");
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		write_log(($cached !== false), $request, "OK");
		return new Vector($response_array);
	}

	public static function predecessors(string $address): Set<string> {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
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

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
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

		if ($response_array->isEmpty() ||$response_array[1] == "500 No predecessors.") {
			write_log(($cached !== false), $request, "NO_PREDECESSORS");
			throw new RuntimeException("The address has no predecessors.");
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		write_log(($cached !== false), $request, "OK");
		return new Set(explode(',', $response_array[0]));
	}

	public static function successors(string $address): Set<string> {
		$redis = RedisWrapper::getRedis();
		$response = "";

		$response_array = Vector {};
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

		if ($cached) {
			$response_array = new Vector(unserialize($cached));
		} else {
			$fp = stream_socket_client("tcp://" . self::$host . ":" . self::$port, self::$errno, self::$errstr, 5);
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

		if ($response_array->isEmpty() || $response_array[1] == "500 No successors.") {
			write_log(($cached !== false), $request, "NO_SUCCESSORS");
			throw new RuntimeException("The address has no successors.");
		}

		try {
			$response_array->pop();
			$response_array->removeKey(0);
		} catch (Exception $e) {}

		write_log(($cached !== false), $request, "OK");
		return new Set(explode(',', $response_array[0]));
	}
}
