import $ from "jquery";
import * as d3 from "d3";
import {context} from "cubism-es";
import {Modal} from 'bootstrap';
// import es6-shim;


let data = {};

let current_voltage_points = []
let server_alive = true;

// Absolutely needed
const isEven = (a) => (a % 2 == 0);
const zeroPad = (num, places) => String(num).padStart(places, '0')

const getData = async () => {
    $.ajax({
        type: "get", url: "/api/data",
        success: function (data, text) {
            let voltages = data["voltages"];
            for (let i = 0; i < voltages.length; i++) {
                let voltage = voltages[i];
                current_voltage_points.push(voltage);
            }
            console.log(voltages);
            let ms = voltages[voltages.length -1];

            let seconds = ms / 1000;
            let hours = parseInt( seconds / 3600 ); // 3,600 seconds in 1 hour
            seconds = seconds % 3600; // seconds remaining after extracting hours
            let minutes = parseInt( seconds / 60 ); // 60 seconds in 1 minute
            seconds = seconds % 60;

            $("#info-time-running").html(zeroPad(hours,2)+":"+zeroPad(minutes,2)+":"+zeroPad(seconds.toFixed(3),3));

        },
        error: function (request, status, error) {
            console.log("Error fetching voltage from API");
            console.table({
                "error": error,
                "status": status
            });
        }
    });
};

let interval = setInterval(getData, 400);

function checkAlive() {
    let serverStatusModel = $("#serverDisconnectedModal");
    

    $.ajax({
        type: "get", url: "/api/alive",
        success: (data, text) => {
            if (!server_alive) {
                console.log("Server connection regained.")
                
                serverStatusModel.hide();
                server_alive = true;
                setInterval(getData, 400);
            }

        },
        error: (request, status, error) => {
            if (server_alive) {
                console.log("Server connection lost.")
                
                serverStatusModel.show();
                server_alive = false;
                clearInterval(getData, 400);
                
                let myModalEl = new Modal($("#serverDisconnectedModal"));
                myModalEl.show();
            }
        }
    });
}

function cubismInitialization() {
    var ctx = context()
        .step(1e4)
        .size(1280);

    d3.select("#voltage-graph-area").selectAll(".axis")
        .data(["top", "bottom"])
        .enter().append("div")
        .attr("class", function(d) { return d + " axis"; })
        .each(function(d) { ctx.axis().ticks(12).orient(d).render(d3.select(this)); });

    const r = d3.select("#voltage-graph-area").append("div")
        .attr("class", "rule");

    ctx.rule().render(r);

    const h = d3.select("#voltage-graph-area").selectAll(".horizon")
        .data(d3.range(1, 10).map(random))
        .enter().insert("div", ".bottom")
        .attr("class", "horizon");
    ctx.horizon()
        .extent([-10, 10])
        .render(h);

    ctx.on("focus", function(i) {
        d3.selectAll(".value").style("right", i == null ? null : ctx.size() - i + "px");
    });

    // Replace this with context.graphite and graphite.metric!
    function random(x) {
        var value = 0,
            values = [],
            i = 0,
            last;
        return ctx.metric(function(start, stop, step, callback) {
            start = +start, stop = +stop;
            if (isNaN(last)) last = start;
            while (last < stop) {
                last += step;
                value = Math.max(-10, Math.min(10, value + .8 * Math.random() - .4 + .2 * Math.cos(i += x * .02)));
                values.push(value);
            }
            callback(null, values = values.slice((start - stop) / step));
        }, x);
    }
}

$(() => {
    setInterval(async () => {
        checkAlive();
    }, 500);
    $.ajax({
        type: "get", url: "/api/device-info",
        success: (data, text) => {
            let virtualChannelCount = data["channel_info"].map((e) => e["virt_channels"]).reduce((a,b) => a + b)
            let ChannelCount = data["channel_info"].length

            $("#info-picoscope-type").html("PicoScope " + data["pico_scope_type"]);
            $("#info-channel-count").html(ChannelCount + " (" + data["channel_info"].map((a) => a["channel"]).join(" | ") + ")");
            $("#info-virtual-channel-count").html(virtualChannelCount);

            $("#info-refresh-rate").html(data["refresh_rate"]+" / "+(data["refresh_rate"]*ChannelCount)/(virtualChannelCount));
            $("#info-voltage-range").html(data["channel_info"].map((e) => e["channel"] + ": " + e["voltage_range"]).join(", "))
        },
        error: (request, status, error) => {
            console.log("Error retrieving device data.");
            console.table({
                "error": error,
                "status": status
            })
        }
    });
    cubismInitialization();
});