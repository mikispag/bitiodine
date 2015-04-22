<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/xhp/init.php");

$title = "BitIodine - Get more from the blockchain";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
    $labels_map = BitIodine::getLabels();
} catch (Exception $e) {
    // Nothing...
}

try {
    list($nodes, $arcs) = BitIodine::stats();
} catch (Exception $e) {
    $nodes = "N/A";
    $arcs = "N/A";
}

$labels_select_from = <select id="from_cluster" name="from_cluster" />;
$labels_select_to = <select id="to_cluster" name="to_cluster" />;
$labels_select_from->appendChild(<option value="" selected="selected"> -- </option>);
$labels_select_to->appendChild(<option value="" selected="selected"> -- </option>);
foreach ($labels_map as $label => $cluster_id) {
    $labels_select_from->appendChild(<option value={$cluster_id}>{$label}</option>);
    $labels_select_to->appendChild(<option value={$cluster_id}>{$label}</option>);
}
$labels_select_from->appendChild(<option disabled="disabled">______________</option>);
$labels_select_from->appendChild(<option value="CUSTOM_CLUSTER">Cluster of →</option>);
$labels_select_to->appendChild(<option disabled="disabled">______________</option>);
$labels_select_to->appendChild(<option value="CUSTOM_CLUSTER">Cluster of →</option>);

$content =
        <div id="main-content">

            <main class="nav-animation-element">

                <div id="bitcoin-logo" class="show">
                    <canvas width="428" height="440" style="width: 214px; height: 220px"></canvas>
                </div>

                <section class="show">
                    <h1>Get <span class="gold">more</span> from the <span class="gold">blockchain</span>.</h1>
                    <p>
                        With <strong class="gold">BitIodine</strong> you can <strong>find transactions</strong> between two addresses or two clusters, address-to-cluster and cluster-to-address, <strong>get a list of addresses</strong> that <strong>sent/received Bitcoin to/from a particular address</strong> and visualize <strong>clusters</strong> controlled by the same user or entity, filtering by <strong>amount</strong> and <strong>time</strong>.
                    </p>


                    <form>
                        {$labels_select_from}<span id="from_address_toggle"> <span id="from_or">or</span> 
                        <input type="text" class="center" id="from_address" placeholder="18iEz617DoDp8CNQUyyrjCcC7XCGDf5SVb" value="18iEz617DoDp8CNQUyyrjCcC7XCGDf5SVb" /></span>
                        <br class="clear" />
                        <button class="button" id="path_button">&rarr;&nbsp;฿&nbsp;&rarr;</button>
                        <br class="clear" />
                        {$labels_select_to}<span id="to_address_toggle"> <span id="to_or">or</span>
                        <input type="text" class="center" id="to_address" placeholder="1MhxtR7FojcbBnfni1wDiJ9nBZtTH6nfia" value="1MhxtR7FojcbBnfni1wDiJ9nBZtTH6nfia" /></span>
                        <br class="clear" />
                        <span class="gold">฿</span>
                        <br class="clear" />
                        <div id="amounts"><input type="text" class="center value gold" id="min_value" placeholder="-" /><span class="gold">to</span><input type="text" class="center value gold" id="max_value" placeholder="-" /></div>
                        <br class="clear" />
                        <div id="times"><input type="text" class="center time green" value="" id="min_time" placeholder="no time limit" /><span class="green">to</span><input type="text" class="center time green" value="" id="max_time" placeholder="no time limit" /></div>
                        <input type="hidden" name="min_time" value="0" />
                        <input type="hidden" name="max_time" value="2147483647" />
                    </form>

                    <p>
                        Get <strong>addresses</strong> that sent/received Bitcoin to/from a particular <strong>address</strong>, or find out which other <strong>addresses</strong> are likely to be <strong>controlled</strong> by the same <strong>person</strong> or <strong>entity</strong>.
                    </p>

                    <form>
                        <button class="button small_button" id="predecessors_button">&rarr;฿</button>
                        <input type="text" class="center" id="single_address" placeholder="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" value="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" />
                        <button class="button small_button" id="successors_button">฿&rarr;</button>
                        <br class="clear" />
                        <button class="button" id="cluster_button">Cluster</button>
                    </form>

                    <p class="small">© - a project by <a href="https://miki.it" target="_blank">Michele Spagnuolo</a> (<a href="https://github.com/mikispag/bitiodine" target="_blank">Github</a>). Blockchain data delayed ~6h. {$nodes} nodes, {$arcs} arcs in the graph.
                    </p>

                </section>

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
