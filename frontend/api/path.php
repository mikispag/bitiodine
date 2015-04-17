<?hh

require("../inc/blockchain.php");
require("../inc/bitiodine.php");
require("../inc/api.php");

if (isset($_GET['show_balances']) && intval($_GET['show_balances']) == 1) {
    $show_balances = TRUE;
} else $show_balances = FALSE;

if (!isset($_GET['from']) || !isset($_GET['to'])) {
    die(json_error(500, "Invalid addresses."));
} else {
    $from = trim($_GET['from']);
    $to = trim($_GET['to']);
    $error_code = 500;

    try {
        list($distance, $address_path, $tx_path) = BitIodine::shortest_path_A2A($from, $to);
    } catch (Exception $e) {
        $error_message = $e->getMessage();
        if ($e->getCode() == 404) {
            $error_code = 404;
        }
    }

    if (!isset($error_message)) {
        $tx_values = BlockChain::get_tx_values($tx_path, $address_path);
        if ($show_balances)
            $balances = BlockChain::get_balances($address_path);
    } else {
        die(json_error($error_code, $error_message));
    }
}

$response = array();

foreach ($address_path as $i => $address) {
    if ($i < count($address_path) - 1) {
        $response["path"][$i] = array(
            "n"             =>      $i,
            "address"       =>      array(
                "address"       =>      $address,
                ),
            "transaction"   =>      array(
                "tx_hash"           =>      $tx_path[$i],
                "value"             =>      number_format($tx_values[$tx_path[$i]], 8),
                "input_address"     =>      $address,
                "output_address"    =>      $address_path[$i+1]
                )
        );
    } else {
        $response["path"][$i] = array(
            "n"             =>      $i,
            "address"       =>      array(
                "address"       =>      $address,
                )
        );
    }
}

if ($show_balances) {
    for ($i = 0; $i < count($response["path"]); $i++) {
        $response["path"][$i]["address"]["balance"] = $balances[$address_path[$i]];
    }
}

print json_return($response);
