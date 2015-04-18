<?hh

$head =
<head>
    <meta charset="utf-8" />
    <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
    <meta name="keywords" content="bitiodine,blockchain analysis,bitcoin forensics" />
    <meta name="description" content="With BitIodine you can find connecting paths between two addresses, visualize clusters controlled by the same user or entity, and get insights about activity on the network." />
    <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1" />
    <meta http-equiv="X-UA-Compatible" content="IE=edge" />
    <meta property="og:image" content="/img/og.png" />
    <link rel="image_src" type="image/png" href="/img/og.png" />
    <link rel="icon" type="image/png" href="/favicon.png" />

    <link rel="stylesheet" href="/css/fonts.css" />
    <link rel="stylesheet" href="/css/style.css" />
    <link rel="stylesheet" href="/css/bitcoin.css" />

    <!--[if lt IE 9]>
    <script src="/js/html5shiv.js"></script>
    <![endif]-->

    <script src="/js/bitiodine.js"></script>
    <script src={"/js/" . basename($_SERVER['SCRIPT_FILENAME'], ".php") . "_jquery.js"}></script>

    <title>{$title}</title>
</head>;

if (isset($usd_price)) {
    $price_widget =                        
                        <li id="flags" class="more">
                            <span ><img src="/img/flags/us.png" class="flag" alt="USD" />$ {number_format($usd_price, 2)}</span>
                            <ul>
                                <li><img src="/img/flags/eu.png" class="flag" alt="EUR" />€ {number_format($eur_price, 2)}</li>
                                <li><img src="/img/flags/gb.png" class="flag" alt="GBP" />£ {number_format($gbp_price, 2)}</li>
                                <li><img src="/img/flags/jp.png" class="flag" alt="JPY" />¥ {number_format($jpy_price, 2)}</li>
                            </ul>
                        </li>;
} else $price_widget = <li />;

$header =
        <header id="main-header" class="transparent show">
            <div class="inner-col">
                <nav>
                    <ul class="pages">
                        <li><a href="/">Home</a>
                        </li>
                        <li><a href="/path">Find paths</a>
                        </li>
                        <li><a href="/cluster">See clusters</a>
                        </li>
                        <li><a href="#" class="soon">Get insights</a>
                        </li>
                        {$price_widget}
                    </ul>
                    <ul class="external">
                        <li><a href="https://miki.it/articles/papers/#bitiodine"><span>About</span></a>
                        </li>
                        <li><a href="/api"><span>API</span></a>
                        </li>
                        <li><a href="&#109;ailto&#58;%&#55;&#51;&#117;%70po%&#55;2%74&#64;%62itiod&#105;ne.n&#37;&#54;5%&#55;4"><span>Contact</span></a>
                        </li>

                        <li class="button">
                            <a href="bitcoin:1N9iYCVz5p8QeZF1wTqZxpxKMdTuaeMYRr">Donate</a>
                        </li>
                    </ul>
                </nav>
            </div>
            <div class="mobile-nav-button"></div>
        </header>;

$analytics =
    <script type="text/javascript" src="/js/ga.js" defer="defer"></script>;

$body =
<body>
    <div id="main">
    {$header}
    {$content}
    {$analytics}
    </div>
</body>;

