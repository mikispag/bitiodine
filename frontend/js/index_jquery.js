$(document).ready(function () {
    $("#a2a_button").click(function (e) {
        e.preventDefault();
        if ($("#from_address").val().length > 26 && $("#to_address").val().length > 26) {
            location.href = 'https://bitiodine.net/a2a/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val());
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#a2a_button").html("&rarr;&nbsp;฿&nbsp;&rarr;");
            }, 2000);
        }
    });
    $("#cluster_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            location.href = 'https://bitiodine.net/cluster/' + encodeURIComponent($("#single_address").val());
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#cluster_button").text("Cluster");
            }, 2000);
        }
    });
    $("#predecessors_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            location.href = 'https://bitiodine.net/predecessors/' + encodeURIComponent($("#single_address").val());
        } else {
            $(this).text(":)");
            setTimeout(function() {
                $("#predecessors_button").html("&rarr;฿");
            }, 2000);
        }
    });
    $("#successors_button").click(function (e) {
        e.preventDefault();
        if ($("#single_address").val().length > 26) {
            location.href = 'https://bitiodine.net/successors/' + encodeURIComponent($("#single_address").val());
        } else {
            $(this).text(":)");
            setTimeout(function() {
                $("#successors_button").html("฿&rarr;");
            }, 2000);
        }
    });
    $(".soon").click(function (e) {
        e.preventDefault();
        $(this).text("Soon :)").fadeIn(2000).fadeOut(2000);
    });
});