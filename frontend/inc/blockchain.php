<?hh

class BlockChain {

	private static $HOURS_CACHE = 24;

	public static function getShortAddress(string $address): string {
		return substr($address, 0, 6) . "...";
	}

	public static function updateBTCPrice(): (float, float, float, float) {
		$json = file_get_contents("http://btc.blockr.io/api/v1/exchangerate/current");
		if ($json === FALSE) {
			throw new RuntimeException("Unable to fetch Bitcoin exchange rates.");
		}
		$json_arr = json_decode($json, true);
		$usd_price = 1 / floatval($json_arr["data"][0]["rates"]["BTC"]);
		$eur_price = $usd_price * floatval($json_arr["data"][0]["rates"]["EUR"]);
		$gbp_price = $usd_price * floatval($json_arr["data"][0]["rates"]["GBP"]);
		$jpy_price = $usd_price * floatval($json_arr["data"][0]["rates"]["JPY"]);
		return tuple($usd_price, $eur_price, $gbp_price, $jpy_price);
	}

	public static function getBTCPrice(): (float, float, float, float) {
		$redis = RedisWrapper::getRedis();
		$usd_price = $redis->get('btc_usd');

		if (is_null($usd_price)) {
			try {
				$btc_price = self::updateBTCPrice();
			} catch (Exception $e) {
				throw new RuntimeException("Unable to fetch Bitcoin exchange rates.");
			}
			list($usd_price, $eur_price, $gbp_price, $jpy_price) = $btc_price;
			$redis->set('btc_usd', $usd_price, 3600);
			$redis->set('btc_eur', $eur_price, 3600);
			$redis->set('btc_gbp', $gbp_price, 3600);
			$redis->set('btc_jpy', $jpy_price, 3600);
		}

		$eur_price = $redis->get('btc_eur');
		$gbp_price = $redis->get('btc_gbp');
		$jpy_price = $redis->get('btc_jpy');

		return tuple($usd_price, $eur_price, $gbp_price, $jpy_price);
	}

	public static function get_tx_values_from_list(Vector<string> $tx_path, Vector<string> $address_path): Vector<string> {
		$n_tx = count($tx_path);
		$tx_values = Vector {};
		$tx_chunks = array_chunk($tx_path, 20);
		$tx_id = 0;
		foreach ($tx_chunks as $tx_chunk) {
			$url = "http://btc.blockr.io/api/v1/tx/info/" . implode(",", $tx_chunk);
			$json = file_get_contents($url);
			if ($json === FALSE) {
				throw new RuntimeException("Unable to retrieve TX values.");
			}
			$json_arr = json_decode($json, true);
			for ($i=0; $tx_id < $n_tx; $tx_id++, $i++) {
				if ($n_tx == 1) {
					$vouts_arr = $json_arr["data"]["trade"]["vouts"];
				} else $vouts_arr = $json_arr["data"][$i]["trade"]["vouts"];
				foreach ($vouts_arr as $vout) {
					if ($vout["address"] == $address_path[$tx_id]) {
						$tx_values[] = number_format($vout["amount"], 8);
						break;
					}
				}
			}
		}
		return $tx_values;
	}

	public static function get_tx_values(Vector<string> $tx_path, Vector<string> $address_path): ImmMap<string, string> {
		$redis = RedisWrapper::getRedis();

		$tx_values = Map {};
		$txs_to_query = Vector {};

		$address_path_int = $address_path->toVector();
		$address_path_int->removeKey(0);
		$address_path_clean = $address_path_int->toVector();

		foreach ($tx_path as $index => $tx) {
			$address = $address_path_int[$index];
			$cached = $redis->get("V_" . $tx . "_" . $address);

			if ($cached) {
				$tx_values[$tx] = number_format($cached, 8);
				$address_path_clean->removeKey($index);
			} else {
				$txs_to_query[] = $tx;
			}
		}

		try {
			$txs_queried = self::get_tx_values_from_list($txs_to_query, $address_path_clean);
		} catch (Exception $e) {
			throw new RuntimeException($e->getMessage());
		}

		foreach ($txs_queried as $index => $value) {
			$redis->set("V_" . $txs_to_query[$index] . "_" . $address_path_clean[$index], $value, 3600 * self::$HOURS_CACHE);
			$tx_values[$txs_to_query[$index]] = $value;
		}
		return new ImmMap($tx_values);
	}

	public static function get_balances(Iterable<string> $addresses): ImmMap<string, string> {
		$redis = RedisWrapper::getRedis();
		$balances = array();
		$addresses_to_query = Vector {};

		foreach ($addresses as $address) {
			$cached = $redis->get("B_$address");

			if ($cached) {
				$balances[$address] = number_format($cached, 2);
			} else {
				$addresses_to_query[] = $address;
			}
		}

		$balances_queried = self::get_balances_from_list($addresses_to_query);

		foreach ($balances_queried as $index => $balance) {
			$redis->set("B_" . $addresses_to_query[$index], $balance, 3600 * self::$HOURS_CACHE);
			$balances[$addresses_to_query[$index]] = $balance;		
		}

		arsort($balances);
		return new ImmMap($balances);
	}

	private static function get_balances_from_list(Vector<string> $addresses): Vector<string> {
		$balances = Vector {};
		$addresses_chunks = array_chunk($addresses, 20);
		foreach ($addresses_chunks as $addresses_chunk) {
			$url = "http://btc.blockr.io/api/v1/address/balance/" . implode(",", $addresses_chunk);
			$json = file_get_contents($url);
			if ($json === FALSE) {
				throw new RuntimeException("Unable to retrieve balances.");
			}
			$json_arr = json_decode($json, true);
			for ($balance_id = 0; $balance_id < count($addresses_chunk); $balance_id++) {
				if (!isset($json_arr["data"][0])) {
					$balance = $json_arr["data"]["balance"];
				} else {
					$balance = $json_arr["data"][$balance_id]["balance"];
				}
				$balances[] = number_format($balance, 2);
			}
		}
		return $balances;
	}

}