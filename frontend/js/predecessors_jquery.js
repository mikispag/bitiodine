$(document).ready(function () {
    $("#predecessors_button").click(function (e) {
        e.preventDefault();
        if ($("#predecessors_address").val().length > 26) {
            location.href = 'https://bitiodine.net/predecessors/' + encodeURIComponent($("#predecessors_address").val());
        } else {
            $(this).text(":)");
            setTimeout(function() {
                $("#predecessors_button").html("&rarr;&nbsp;฿");
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