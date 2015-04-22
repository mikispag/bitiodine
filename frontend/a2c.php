<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/security.php");
require("inc/xhp/init.php");

$title = "BitIodine - Address to cluster";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
    $labels_map = BitIodine::getLabels();
} catch (Exception $e) {
    $price_error = TRUE;
}

if (!isset($_GET['from']) || !isset($_GET['to'])) {
    header("Location: https://bitiodine.net/");
} else {
    $from = $_GET['from'];
    $to = $_GET['to'];
    $min_time = 0;
    $max_time = 2147483647;
    $min_value = floatval(0);
    $max_value = INF;

    if (isset($_GET['min_time']) && isset($_GET['max_time'])) {
        $min_time = intval(trim($_GET['min_time']));
        $max_time = intval(trim($_GET['max_time']));
    }
    if (isset($_GET['min_value']) && isset($_GET['max_value'])) {
        $min_value = floatval(trim($_GET['min_value']));
        $max_value = floatval(trim($_GET['max_value']));
    }
    $show_form = FALSE;

    Security::throttle_ip_web();

    try {
        if (!is_numeric($_GET['to'])) {
            $to = BitIodine::cluster_id($to);
        }
        list($tx_hashes, $timestamps, $values) = BitIodine::A2C($from, $to, $min_time, $max_time, $min_value, $max_value);
        $n_tx = $tx_hashes->count();
        $plural_form = ($n_tx > 1) ? "s" : "";

        foreach ($labels_map as $label => $cluster_id) {
            if ($cluster_id == $to) {
                $cluster_label_show = $label;
                break;
            }
        }
    } catch (Exception $e) {
        $error_message = $e->getMessage();
    }

}

$section_show = <section class="show" />;
$cluster_label_show = $to;

if (isset($error_message)) {
    $header_message = "No transactions found :(";
    $subheader = $results = <span />;
    $description_or_error = <p><span class="error">{$error_message}</span></p>;
} else {
    $header_message = "Here are your transactions.";
    $subheader = <p>
                    We found <strong>{$n_tx} transaction{$plural_form}</strong> from <strong>{BlockChain::getShortAddress($from)}</strong> to <a href={"/cluster/" . $_GET['to']}><strong>cluster {$cluster_label_show}</strong></a> matching your criteria.
                </p>;
    $description_or_error = <p>Click on the table header to sort by amount or time.</p>;
    $tbody = <tbody />;        

    foreach ($tx_hashes as $i => $tx) {
        $row = <tr><td><a class="gold mono" href={"https://blockr.io/tx/info/$tx"}>{$tx}</a></td><td data-sort-value={number_format($values[$i], 8)}>{number_format($values[$i], 8)}&nbsp;฿</td><td class="datetime" data-sort-value={$timestamps[$i]}>{$timestamps[$i]}</td></tr>;
        $tbody->appendChild($row);
    }

    $results =  <table id="result_table">
                    <thead>
                        <tr>
                        <th>Transaction hash</th>
                        <th data-sort="float" data-sort-default="desc">Amount</th>
                        <th data-sort="int">Date / Time</th>
                        </tr>
                    </thead>
                    {$tbody}
                </table>;
}

$section_show->appendChild(<h1>{$header_message}</h1>);
$section_show->appendChild($subheader);
$section_show->appendChild($description_or_error);
$section_show->appendChild($results);

$content =
        <div id="main-content">

            <main class="nav-animation-element">

                <div id="bitcoin-logo" class="show">
                    <canvas width="428" height="440" style="width: 214px; height: 220px"></canvas>
                </div>

                {$section_show}

            </main>

        </div>;

include("inc/template/page.php");

echo
    <x:doctype>
    <html lang="en" id="bitcoin" class="tablet mobile js canvas csscolumns cssgradients csstransitions">
    {$head}
    {$body}
    </html>
    </x:doctype>;
