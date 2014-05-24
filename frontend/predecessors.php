<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/security.php");
require("inc/xhp/init.php");

$title = "BitIodine - Find predecessors";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
} catch (Exception $e) {
    $price_error = TRUE;
}
if (!isset($_GET['address'])) {
    $show_form = TRUE;
} else {
    $address = trim($_GET['address']);

    Security::throttle_ip_web();

    try {
        $predecessors = BitIodine::predecessors($address);
        $cluster_size = count($predecessors);
        $plural_form = ($cluster_size > 1) ? "es" : "";
        $balances = BlockChain::get_balances($predecessors);
    } catch (Exception $e) {
        $error_message = $e->getMessage();
    }

}


$section_show = <section class="show" />;

if ($show_form) {
    $section_show->appendChild(<h1>Get predecessors.</h1>);
    $section_show->appendChild(<p>Find out <strong>addresses</strong> that sent Bitcoin to a particular <strong>address</strong>.</p>);
    $section_show->appendChild(
        <form>
            <input type="text" class="center" id="predecessors_address" placeholder="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" value="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" />
            <br class="clear" />
            <button class="button" id="predecessors_button">&rarr;฿</button>
        </form>);
} else {
    if (isset($error_message)) {
        $header_message = "No predecessor found :(";
        $subheader = $results = <span />;
        $description_or_error = <p><span class="error">{$error_message}</span></p>;
    } else {
        $header_message = "Here are the predecessors.";
        $subheader = <p>
                        <strong>{$cluster_size} address{$plural_form}</strong> sent Bitcoin to <strong>{$address}</strong>.
                    </p>;
        $description_or_error = <p>Click on an address to get more information.</p>;
        $results = <p class="orange padded"></p>;

        foreach ($balances as $address => $balance) {
            $results->appendChild(<span><a href={"/predecessors/" . $address}><img src="/img/predecessors.png" class="predecessors" alt="Find predecessors" title="Find predecessors" /></a>&nbsp;<a href={"https://blockr.io/address/info/" . $address}>{$address}</a>&nbsp;<a href={"/successors/" . $address}><img src="/img/successors.png" class="successors" alt="Find successors" title="Find successors" /></a>&nbsp;<a href={"/cluster/" . $address}><img src="/img/cluster.png" width="16" height="16" alt="Cluster address" title="Cluster address" /></a>&nbsp;<span class="gold">(฿ {$balance})</span></span>);
            $results->appendChild(<br />);
        }
    }

    $section_show->appendChild(<h1>{$header_message}</h1>);
    $section_show->appendChild($subheader);
    $section_show->appendChild($description_or_error);
    $section_show->appendChild($results);
}

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
