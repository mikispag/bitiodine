<?hh

require("../inc/blockchain.php");
require("../inc/bitiodine.php");
require("../inc/api.php");

if (isset($_GET['show_balances']) && intval($_GET['show_balances']) == 1) {
    $show_balances = TRUE;
} else $show_balances = FALSE;

if (isset($_GET['steps'])) {
    $show_balances = FALSE;
    $steps = abs(intval($_GET['steps']));
    if ($steps > 2) {
        die(json_error(500, "Maximum recursion steps: 2. Contact support@bitiodine.net if you absolutely need more."));
    }
} else $steps = 0;

if (!isset($_GET['address'])) {
    die(json_error(500, "Invalid addresses."));
}

$address = trim($_GET['address']);
$error_code = 500;

try {
    $predecessors = BitIodine::predecessors($address);
} catch (Exception $e) {
    $error_message = $e->getMessage();
    if ($e->getCode() == 404) {
        $error_code = 404;
    }
}

if (!isset($error_message)) {
    if ($show_balances)
        $balances = BlockChain::get_balances($predecessors);
} else {
    die(json_error($error_code, $error_message));
}

$addresses = $predecessors;

for (; $steps > 0; $steps--) {
    $to_be_added = new Set<String>;
    foreach ($addresses as $address) {
        try {
            $to_be_added->addAll(BitIodine::predecessors($address));
        } catch (Exception $e) {
            // Do nothing
        }
    }
    $addresses = $to_be_added;
    $predecessors->addAll($to_be_added);
}

$response = array();
foreach ($predecessors as $address) {
        $response[] = array(
            "address"       =>      array(
                "address"       =>      $address,
                )
        );
}

if ($show_balances) {
    for ($i = 0; $i < count($response); $i++) {
        $response[$i]["address"]["balance"] = $balances[$predecessors[$i]];
    }
}

print json_return($response);
