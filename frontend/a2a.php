<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/security.php");
require("inc/xhp/init.php");

$title = "BitIodine - Find paths";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
} catch (Exception $e) {
    $price_error = TRUE;
}

if (!isset($_GET['from']) || !isset($_GET['to'])) {
    $show_form = TRUE;
} else {
    $from = trim($_GET['from']);
    $to = trim($_GET['to']);
    $show_form = FALSE;

    Security::throttle_ip_web();

    try {
        list($distance, $address_path, $tx_path) = BitIodine::A2A($from, $to);
        $plural_form = ($distance > 1) ? "s" : "";
        $tx_values = BlockChain::get_tx_values($tx_path, $address_path);
        $balances = BlockChain::get_balances($address_path);
    } catch (Exception $e) {
        $error_message = $e->getMessage();
    }

}

$section_show = <section class="show" />;

if ($show_form) {
    $section_show->appendChild(<h1>Find connecting paths.</h1>);
    $section_show->appendChild(<p>Get details about <strong>shortest paths</strong> between <strong>two addresses</strong>.</p>);
    $section_show->appendChild(
        <form>
            <input type="text" class="center" id="from_address" placeholder="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" value="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" />
            <button class="button" id="a2a_button">&rarr;&nbsp;฿&nbsp;&rarr;</button>
            <input type="text" class="center" id="to_address" placeholder="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" value="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" />
            <br class="clear" />
        </form>);
} else {
    if (isset($error_message)) {
        $header_message = "No path found :(";
        $subheader = $results = <span />;
        $description_or_error = <p><span class="error">{$error_message}</span></p>;
    } else {
        $header_message = "Here's your path.";
        $subheader = <p>
                        Shortest paths from <strong>{BlockChain::getShortAddress($from)}</strong> to <strong>{BlockChain::getShortAddress($to)}</strong> are <strong>{$distance} hop{$plural_form}</strong> (<em>transaction{$plural_form}</em>).
                    </p>;
        $description_or_error = <p>Click on the arrows between two addresses to get details on a transaction.</p>;
        $results = <p class="orange padded"></p>;
        foreach ($address_path as $i => $address) {
            if ($i > 0) {
                $tx_arrow = <span><a class="gold" href={"https://blockr.io/tx/info/" . $tx_path[$i-1]}>&rarr;&nbsp;{number_format($tx_values[$tx_path[$i-1]], 8)}&nbsp;฿&nbsp;&rarr;</a></span>;
                $results->appendChild($tx_arrow);
                $results->appendChild(<br />);
            }
            $address_line = <span><a href={"/predecessors/" . $address}><img class="predecessors" src="/img/predecessors.png" alt="Find predecessors" title="Find predecessors" /></a>&nbsp;<a href={"https://blockr.io/address/info/" . $address}>{$address}</a> <span class="gold">(฿ {$balances[$address]})</span>&nbsp;<a href={"/successors/" . $address}><img class="successors" src="/img/successors.png" alt="Find successors" title="Find successors" /></a>&nbsp;<a href={"/cluster/" . $address}><img class="cluster" src="/img/cluster.png" alt="Cluster address" title="Cluster address" /></a></span>;
            $results->appendChild($address_line);
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
