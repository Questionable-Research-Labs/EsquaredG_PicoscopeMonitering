import $ from "jquery";
import * as d3 from "d3";
import { context } from "cubism-es";
import { Modal } from 'bootstrap';

let data = {};

let current_voltage_points = {}
let server_alive = true;

// Absolutely needed
const isEven = (a) => (a % 2 == 0);
const zeroPad = (num, places) => String(num).padStart(places, '0')



const getData = async () => {
    $.ajax({
        type: "get", url: "/api/data",
        success: function (data, text) {
            let voltages = data["voltages"];
            if (Object.keys(voltages).length != 0) {
                console.log(voltages)
                for (let channel of Object.keys(voltages)) {
                    current_voltage_points[channel]+=voltages[channel]
                    console.log(current_voltage_points)
                }
                
                
                let top_channel = voltages[Object.keys(voltages)[0]];

                let ms = top_channel[top_channel.length - 1][1];

                let seconds = ms / 1000;
                let hours = parseInt(seconds / 3600); // 3,600 seconds in 1 hour
                seconds = seconds % 3600; // seconds remaining after extracting hours
                let minutes = parseInt(seconds / 60); // 60 seconds in 1 minute
                seconds = seconds % 60;

                $("#info-time-running").html(zeroPad(hours, 2) + ":" + zeroPad(minutes, 2) + ":" + zeroPad(seconds.toFixed(3), 3));
            } else {
                console.log("No voltages were retrieved.")
            }


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

let deviceConfig = {};

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
        .step(30)
        .size($("#voltage-graph-area").width());

    d3.select("#voltage-graph-area").selectAll(".axis")
        .data(["top", "bottom"])
        .enter().append("div")
        .attr("class", function (d) { return d + " axis"; })
        .each(function (d) { ctx.axis().ticks(12).orient(d).render(d3.select(this)); });

    const r = d3.select("#voltage-graph-area").append("div")
        .attr("class", "rule");

    ctx.rule().render(r);
    console.log(deviceConfig["channel_info"])
    const h = d3.select("#voltage-graph-area").selectAll(".horizon")
        .data(d3.range(0, Object.keys(current_voltage_points).length).map(generateGraphPoints))
        .enter().insert("div", ".bottom")
        .attr("class", "horizon");
    const range = Math.max(...deviceConfig["channel_info"].map((e) => e["voltage_range"]))
    ctx.horizon()
        .extent([-range, range])
        .height($("#voltage-graph-wrapper").height() / deviceConfig["channel_info"].length)
        .render(h);

    ctx.on("focus", function (i) {
        d3.selectAll(".value").style("right", i == null ? null : ctx.size() - i + "px");
    });

    // Replace this with context.graphite and graphite.metric!
    function generateGraphPoints(x) {
        return ctx.metric(function (start, stop, step, callback) {
            callback(null, x);
        }, x);
    }
}

$(() => {
    setInterval(async () => {
        checkAlive();
    }, 500);
    getDeviceInfo();
});

function getDeviceInfo() {
    $.ajax({
        type: "get", url: "/api/device-info",
        success: (data, text) => {
            deviceConfig = data;
            let virtualChannelCount = data["channel_info"].map((e) => e["virt_channels"]).reduce((a, b) => a + b)
            let ChannelCount = data["channel_info"].length

            $("#info-picoscope-type").html("PicoScope " + data["pico_scope_type"]);
            $("#info-channel-count").html(ChannelCount + " (" + data["channel_info"].map((a) => a["channel"]).join(" | ") + ")");
            $("#info-virtual-channel-count").html(virtualChannelCount);

            $("#info-refresh-rate").html(data["refresh_rate"] + " / " + (data["refresh_rate"] * ChannelCount) / (virtualChannelCount));
            $("#info-voltage-range").html(data["channel_info"].map((e) => e["channel"] + ": " + e["voltage_range"]).join(", "))
            cubismInitialization();
        },
        error: (request, status, error) => {
            console.log("Error retrieving device data.");
            console.table({
                "error": error,
                "status": status
            });
            getDeviceInfo()
        }
    });
}