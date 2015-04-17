$(document).ready(function () {
    $("#path_button").click(function (e) {
        e.preventDefault();
        if ($("#from_address").val().length > 26 && $("#to_address").val().length > 26) {
            $("#from").text($("#from_address").val());
            $("#to").text($("#to_address").val());
            $("#path").prop("href",  '/api/path/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val()));
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#path_button").html("&rarr;&nbsp;฿&nbsp;&rarr;");
            }, 2000);
        }
    });
    $("#predecessors_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            $("#operation").text("predecessors");
            $("#address").text($("#single_address").val());
            $("#predsucc").prop("href",  '/api/predecessors/' + encodeURIComponent($("#single_address").val()));
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#predecessors_button").html("&rarr;฿&nbsp;");
            }, 2000);
        }
    });
    $("#successors_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            $("#operation").text("successors");
            $("#address").text($("#single_address").val());
            $("#predsucc").prop("href",  '/api/successors/' + encodeURIComponent($("#single_address").val()));
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#successors_button").html("฿&rarr;");
            }, 2000);
        }
    });
    $("#cluster_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            $("#operation").text("cluster");
            $("#address_cluster").text($("#single_address").val());
            $("#predsucc").prop("href",  '/api/cluster/' + encodeURIComponent($("#single_address").val()));
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#cluster_button").html("฿&rarr;");
            }, 2000);
        }
    });
    $(".soon").click(function (e) {
        e.preventDefault();
        $(this).text("Soon :)").fadeIn(2000).fadeOut(2000);
    });
});