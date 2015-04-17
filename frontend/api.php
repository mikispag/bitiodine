<?hh

require("inc/blockchain.php");
require("inc/bitiodine.php");
require("inc/xhp/init.php");

$title = "BitIodine - API";

try {
    list($usd_price, $eur_price, $gbp_price, $jpy_price) = BlockChain::getBTCPrice();
} catch (Exception $e) {
    $price_error = TRUE;
}

try {
    list($nodes, $arcs) = BitIodine::stats();
} catch (Exception $e) {
    $error_message = $e->getMessage();
}

$content =
        <div id="main-content">

            <main class="nav-animation-element">

                <div id="bitcoin-logo" class="show">
                    <canvas width="428" height="440" style="width: 214px; height: 220px"></canvas>
                </div>

                <section class="show">
                    <h1>Get <span class="gold">more</span> with our <span class="gold">API</span>.</h1>
                    <p>
                        With our <strong class="gold">API</strong> you can programmatically query <strong class="gold">BitIodine</strong>.
                    </p>

                    <div class="rounded">

                        <p class="white">
                            Get details about <strong>shortest paths</strong> between <strong>two addresses</strong>.
                        </p>

                        <form>
                            <input type="text" class="center" id="from_address" placeholder="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" value="1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE" />
                            <button class="button" id="path_button">&rarr;&nbsp;฿&nbsp;&rarr;</button>
                            <input type="text" class="center" id="to_address" placeholder="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" value="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" />
                            <br class="clear" />
                        </form>

                        <p class="api">
                            <a id="path" target="_blank" href="/api/path/1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa">/api/path/<span id="from">1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE</span>/<span id="to">1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa</span></a>
                        </p>

                        <p class="gold">
                            Parameters
                        </p>

                        <p>
                                <strong class="gold">show_balances</strong> — if set to <span class="gold">1</span> returns balances for each address.
                        </p>

                        <p class="gold">
                            Example
                        </p>

                        <p>
                                <span class="white">/api/path/1AA2MKdGEv7kQZq2KXC5HdQcVaaCS8QcGE/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa?show_balances=1</span>
                                <br />
                                — returns a shortest path from <span class="gold">1AA...</span> to <span class="gold">1A1...</span>.
                        </p>

                    </div>
                    <div class="rounded">

                        <p class="white">
                            Get <strong>addresses</strong> that sent/received Bitcoin to/from a particular <strong>address</strong>, or find out which other <strong>addresses</strong> are likely to be <strong>controlled</strong> by the same <strong>person</strong> or <strong>entity</strong>.
                        </p>

                        <form>
                            <button class="button small_button" id="predecessors_button">&rarr;฿</button>
                            <input type="text" class="center" id="single_address" placeholder="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" value="1Shremdh9tVop1gxMzJ7baHxp6XX2WWRW" />
                            <button class="button small_button" id="successors_button">฿&rarr;</button>
                            <br class="clear" />
                            <button class="button" id="cluster_button">Cluster</button>
                        </form>

                        <p class="api">
                            <a id="predsucc" target="_blank" href="/api/predecessors/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa">/api/<span id="operation">predecessors</span>/<span id="address">1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa</span></a>
                        </p>

                        <p class="gold">
                            Parameters
                        </p>

                        <p>
                                <strong class="gold">show_balances</strong> — if set to <span class="gold">1</span> returns balances for each address.
                        </p>

                        <p class="gold">
                            Examples
                        </p>

                        <p>
                                <span class="white">/api/predecessors/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa?show_balances=1</span>
                                <br />
                                — returns predecessors for <span class="gold">1A1...</span> showing balances for each predecessor.
                                <br />
                                <span class="white">/api/successors/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa</span>
                                <br />
                                — returns successors for <span class="gold">1A1...</span> without showing balances.
                                <br />
                                <span class="white">/api/cluster/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa?show_balances=1</span>
                                <br />
                                — returns a cluster for <span class="gold">1A1...</span> showing balances for each address in the cluster.
                        </p>

                    </div>
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