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
    $(".soon").click(function (e) {
        e.preventDefault();
        $(this).text("Soon :)").fadeIn(2000).fadeOut(2000);
    });
    $(document).find(".gold").fadeOut(100).fadeIn(500);
    $(document).find(".predecessors").fadeOut(100).fadeIn(500);
    $(document).find(".successors").fadeOut(100).fadeIn(1500);
    $(document).find(".cluster").fadeOut(100).fadeIn(2000);
});