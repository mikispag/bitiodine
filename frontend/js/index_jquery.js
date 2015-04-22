$(document).ready(function () {
    $('#min_time').datetimepicker({
    dayOfWeekStart : 1,
    lang: 'en',
    minDate: '2009/01/01',
    maxDate: '0',
    onChangeDateTime: function(date) {
        if (!date) {
            $("input[name='min_time']").val(0);
            return;
        }
        var timestamp = Math.round(date.getTime()/1000);
        $("input[name='min_time']").val(timestamp);
    },
    });
    $('#max_time').datetimepicker({
    dayOfWeekStart : 1,
    lang: 'en',
    minDate: '2009/01/01',
    maxDate: '0',
    onChangeDateTime: function(date) {
        if (!date) {
            $("input[name='max_time']").val(2147483647);
            return;
        }
        var timestamp = Math.round(date.getTime()/1000);
        $("input[name='max_time']").val(timestamp);
    },
    });
    $("#min_value").change(function (e) {
        var n = $(this).val();
        if (!n || !(/^([0-9]+(\.[0-9]+)?)$/.test(n))) {
            $(this).val(0);
        } else if (!(/^([0-9]+(\.[0-9]{1,8})?)$/.test(n))) {
            $(this).val(parseFloat($(this).val()).toFixed(8));
        }
    });
    $("#max_value").change(function (e) {
        var n = $(this).val();
        if (!n || !(/^([0-9]+(\.[0-9]+)?)$/.test(n))) {
            $(this).val(10000);
        } else if (!(/^([0-9]+(\.[0-9]{1,8})?)$/.test(n))) {
            $(this).val(parseFloat($(this).val()).toFixed(8));
        }
    });
    if ($("#from_cluster").val() && $("#from_cluster").val() != "CUSTOM_CLUSTER") {
        $("#from_address_toggle").hide();
    }
    if ($("#to_cluster").val() && $("#to_cluster").val() != "CUSTOM_CLUSTER") {
        $("#to_address_toggle").hide();
    }
    $("#from_cluster").change(function (e) {
        if ($(this).val() && $("#from_cluster").val() != "CUSTOM_CLUSTER") {
            $("#from_address_toggle").hide();
        } else {
            $("#from_address_toggle").show();
        }
        if ($("#from_cluster").val() == "CUSTOM_CLUSTER") {
            $("#from_or").toggle();
        }
    });
    $("#to_cluster").change(function (e) {
        if ($(this).val() && $("#to_cluster").val() != "CUSTOM_CLUSTER") {
            $("#to_address_toggle").hide();
        } else {
            $("#to_address_toggle").show();
        }
        if ($("#to_cluster").val() == "CUSTOM_CLUSTER") {
            $("#to_or").toggle();
        }
    });
    $("#path_button").click(function (e) {
        e.preventDefault();
        var min_value = parseFloat($("#min_value").val());
        var max_value = parseFloat($("#max_value").val());

        var url_append = '?';

        if (!(isNaN(min_value)) && isNaN(max_value)) {
            max_value = 21000000;
        }
        if (isNaN(min_value) && !(isNaN(max_value))) {
            min_value = 0;
        }
        if (!(isNaN(min_value)) && !(isNaN(max_value))) {
            if (min_value >= max_value) {
                $(this).text("Bad amounts!");
                $("#amounts").fadeIn(100).fadeOut(100).fadeIn(100).fadeOut(100).fadeIn(100);
                $("#min_value").focus();
                setTimeout(function() {
                    $("#path_button").html("&rarr;&nbsp;฿&nbsp;&rarr;");
                }, 2000);
                return false;
            }
            url_append += 'min_value=' + min_value.toFixed(8) + '&max_value=' + max_value.toFixed(8) + '&';
        }

        var min_time = parseInt($("input[name='min_time']").val());
        var max_time = parseInt($("input[name='max_time']").val());

        if (!(isNaN(min_time)) && !(isNaN(max_time))) {
            if (min_time >= max_time) {
                $(this).text("Bad dates!");
                $("#times").fadeIn(100).fadeOut(100).fadeIn(100).fadeOut(100).fadeIn(100);
                $("#min_time").focus();
                setTimeout(function() {
                    $("#path_button").html("&rarr;&nbsp;฿&nbsp;&rarr;");
                }, 2000);
                return false;
            }
            url_append += 'min_time=' + min_time + '&max_time=' + max_time + '&';
        }
        if ($("#from_cluster").val() == "CUSTOM_CLUSTER" && $("#from_address").val().length > 26) {
            if ($("#to_cluster").val() == "CUSTOM_CLUSTER" && $("#to_address").val().length > 26) {
                location.href = 'https://bitiodine.net/c2c/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val()) + url_append;
                return;
            }
            if ($("#to_cluster").val()) {
                location.href = 'https://bitiodine.net/c2c/' + encodeURIComponent($("#from_address").val()) + '/' + $("#to_cluster").val() + url_append;
                return;
            }
            if ($("#to_address").val().length > 26) {
                location.href = 'https://bitiodine.net/c2a/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val()) + url_append
                return;
            }
        }
        if ($("#to_cluster").val() == "CUSTOM_CLUSTER" && $("#to_address").val().length > 26) {
            if ($("#from_cluster").val()) {
                location.href = 'https://bitiodine.net/c2c/' + $("#from_cluster").val() + '/' + encodeURIComponent($("#to_address").val()) + url_append;
                return;
            }
            if ($("#from_address").val().length > 26) {
                location.href = 'https://bitiodine.net/a2c/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val()) + url_append;
                return;
            }
        }
        if ($("#from_cluster").val() && $("#to_cluster").val()) {
            location.href = 'https://bitiodine.net/c2c/' + $("#from_cluster").val() + '/' + $("#to_cluster").val() + url_append;
            return;
        }
        else if ($("#from_cluster").val() && !$("#to_cluster").val() && $("#to_address").val().length > 26) {
            location.href = 'https://bitiodine.net/c2a/' + $("#from_cluster").val() + '/' + encodeURIComponent($("#to_address").val()) + url_append;
            return;
        }
        else if (!$("#from_cluster").val() && $("#to_cluster").val() && $("#from_address").val().length > 26) {
            location.href = 'https://bitiodine.net/a2c/' + encodeURIComponent($("#from_address").val()) + '/' + $("#to_cluster").val() + url_append;
            return;
        }
        else if (!$("#from_cluster").val() && !$("#to_cluster").val() && $("#from_address").val().length > 26 && $("#to_address").val().length > 26) {
            location.href = 'https://bitiodine.net/a2a/' + encodeURIComponent($("#from_address").val()) + '/' + encodeURIComponent($("#to_address").val()) + url_append;
            return;
        } else {
            $(this).text("Try again :)");
            setTimeout(function() {
                $("#path_button").html("&rarr;&nbsp;฿&nbsp;&rarr;");
            }, 2000);
            return false;
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