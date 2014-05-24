$(document).ready(function () {
    $("#cluster_button").click(function (e) {
        e.preventDefault();
        if ($("#cluster_address").val().length > 26) {
            location.href = 'https://bitiodine.net/cluster/' + encodeURIComponent($("#cluster_address").val());
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#cluster_button").text("Cluster");
            }, 2000);
        }
    });
    $(".soon").click(function (e) {
        e.preventDefault();
        $(this).text("Soon :)").fadeIn(2000).fadeOut(2000);
    });
    $(document).find(".gold").fadeOut(100).fadeIn(500);
    $(document).find(".predecessors").fadeOut(100).fadeIn(500);
    $(document).find(".successors").fadeOut(100).fadeIn(1500);
    $(document).find(".cluster").fadeOut(100).fadeIn(2000);
});