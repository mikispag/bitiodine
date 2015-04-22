<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/security.php");
require("inc/xhp/init.php");

$title = "BitIodine - Find clusters";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
} catch (Exception $e) {
    $price_error = TRUE;
}
if (!isset($_GET['address'])) {
    $show_form = TRUE;
} else {
    $address = $_GET['address'];

    Security::throttle_ip_web();

    if (is_numeric($address)) {
        try {
            $neighbors = BitIodine::print_cluster(intval($address));
            $cluster_size = $neighbors->count();
            $plural_form = ($cluster_size > 1) ? "es" : "";
        } catch (Exception $e) {
            $error_message = $e->getMessage();
        }
    } else {
        try {
            $neighbors = BitIodine::neighbors($address);
            $cluster_size = $neighbors->count();
            $plural_form = ($cluster_size > 1) ? "es" : "";
        } catch (Exception $e) {
            $error_message = $e->getMessage();
        }
    }

    if (!isset($error_message) && $cluster_size < 201) {
        $balances = BlockChain::get_balances($neighbors);
    }

}

$section_show = <section class="show" />;

if ($show_form) {
    $section_show->appendChild(<h1>Get clusters.</h1>);
    $section_show->appendChild(<p>Find out which other <strong>addresses</strong> are likely to be <strong>controlled</strong> by the same <strong>person</strong> or <strong>entity</strong></p>);
    $section_show->appendChild(
        <form>
            <input type="text" class="center" id="cluster_address" placeholder="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" value="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" />
            <br class="clear" />
            <button class="button" id="cluster_button">Cluster</button>
        </form>);
} else {
    if (isset($error_message)) {
        $header_message = "No cluster found :(";
        $subheader = $results = <span />;
        $description_or_error = <p><span class="error">{$error_message}</span></p>;
    } else {
        $header_message = "Here's your cluster.";
        $subheader = <p>
                        The entity owning<br /><strong>{$address}</strong><br />likely controls <strong>{$cluster_size} address{$plural_form}</strong>
                    </p>;
        $description_or_error = <p>Click on an address to get more information.</p>;
        $results = <p class="orange padded"></p>;

        if (isset($balances)) {
            foreach ($balances as $address => $balance) {
                $results->appendChild(<span><a href={"/predecessors/" . $address}><img src="/img/predecessors.png" class="predecessors" alt="Find predecessors" title="Find predecessors" /></a>&nbsp;<a href={"https://blockr.io/address/info/" . $address}>{$address}</a>&nbsp;<a href={"/successors/" . $address}><img src="/img/successors.png" class="successors" alt="Find successors" title="Find successors" /></a>&nbsp;<a href={"/cluster/" . $address}><img src="/img/cluster.png" width="16" height="16" alt="Cluster address" title="Cluster address" /></a>&nbsp;<span class="gold">(฿ {$balance})</span></span>);
                $results->appendChild(<br />);
            }
        } else {
            foreach ($neighbors as $address) {
                $results->appendChild(<span><a href={"/predecessors/" . $address}><img src="/img/predecessors.png" class="predecessors" alt="Find predecessors" title="Find predecessors" /></a>&nbsp;<a href={"https://blockr.io/address/info/" . $address}>{$address}</a>&nbsp;<a href={"/successors/" . $address}><img src="/img/successors.png" class="successors" alt="Find successors" title="Find successors" /></a>&nbsp;<a href={"/cluster/" . $address}><img src="/img/cluster.png" width="16" height="16" alt="Cluster address" title="Cluster address" /></a></span>);
                $results->appendChild(<br />);
            }
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
