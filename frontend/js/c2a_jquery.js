$(document).ready(function () {
    $(".soon").click(function (e) {
        e.preventDefault();
        $(this).text("Soon :)").fadeIn(2000).fadeOut(2000);
    });
    $(document).find(".gold").fadeOut(100).fadeIn(500);

    $(".datetime").each(function() {
        $current_text = $(this).text();
        $(this).text($.format.toBrowserTimeZone(new Date($current_text * 1000), 'MMM d, yyyy h:mm:ss a'));
    });

    var $table = $("#result_table").stupidtable();
    var $th_to_sort = $table.find("thead th").eq(1);
    $th_to_sort.stupidsort();
});