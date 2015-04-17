<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/xhp/init.php");

$title = "BitIodine - Get more from the blockchain";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
} catch (Exception $e) {
    // Nothing...
}

try {
    list($nodes, $arcs) = BitIodine::stats();
} catch (Exception $e) {
    $nodes = "N/A";
    $arcs = "N/A";
}

$content =
        <div id="main-content">

            <main class="nav-animation-element">

                <div id="bitcoin-logo" class="show">
                    <canvas width="428" height="440" style="width: 214px; height: 220px"></canvas>
                </div>

                <section class="show">
                    <h1>Get <span class="gold">more</span> from the <span class="gold">blockchain</span>.</h1>
                    <p>
                        With <strong class="gold">BitIodine</strong> you can find <strong>connecting paths</strong> between two addresses, visualize <strong>clusters</strong> controlled by the same user or entity, and <strong>get insights</strong> about activity on the network.
                    </p>

                    <p>
                        Get details about <strong>shortest paths</strong> between <strong>two addresses</strong>.
                    </p>

                    <form>
                        <input type="text" class="center" id="from_address" placeholder="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" value="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" />
                        <button class="button" id="path_button">&rarr;&nbsp;฿&nbsp;&rarr;</button>
                        <input type="text" class="center" id="to_address" placeholder="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" value="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" />
                        <br class="clear" />
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

                </section>

            </main>


            <footer id="main-footer" class="show nav-animation-element">
                <div class="inner-col">
                    <p>© - a project by <a href="https://miki.it" target="_blank">Michele Spagnuolo</a> (<a href="https://github.com/mikispag/bitiodine" target="_blank">Github</a>). Blockchain data delayed ~6h. {$nodes} nodes, {$arcs} arcs in the graph.
                    </p>
                </div>
            </footer>
        </div>;

include("inc/template/page.php");

echo
    <x:doctype>
    <html lang="en" id="bitcoin" class="tablet mobile js canvas csscolumns cssgradients csstransitions">
    {$head}
    {$body}
    </html>
    </x:doctype>;
