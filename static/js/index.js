import $ from "jquery";
import { Modal } from "bootstrap";
import { Chart } from "chart.js";
import 'chartjs-plugin-streaming';
import moment from 'moment';

let data = {};
let graph;

let current_voltage_points = {};
let server_alive = true;
let deviceConfig = {};

// Absolutely needed
const isEven = (a) => a % 2 == 0;
const zeroPad = (num, places) => String(num).padStart(places, "0");

const getData = async () => {
    $.ajax({
        type: "get",
        url: "/api/data",
        success: function (data, text) {
            let voltages = data["voltages"];
            if (Object.keys(voltages).length != 0) {
                console.log("Voltages", voltages);
                for (let channel of Object.keys(voltages)) {
                    console.log("Channel", voltages[channel]);
                    if (current_voltage_points[channel] === undefined) {
                        current_voltage_points[channel] = [];
                    }
                    current_voltage_points[channel] = current_voltage_points[
                        channel
                    ].concat(
                        voltages[channel].map((v) => {
                          return { y: v[0], x: v[2]};
                        })
                    );
                    console.log("CVP", current_voltage_points);
                }

                let top_channel = voltages[Object.keys(voltages)[0]];

                let ms = top_channel[top_channel.length - 1][1];

                let seconds = ms / 1000;
                let hours = parseInt(seconds / 3600); // 3,600 seconds in 1 hour
                seconds = seconds % 3600; // seconds remaining after extracting hours
                let minutes = parseInt(seconds / 60); // 60 seconds in 1 minute
                seconds = seconds % 60;

                $("#info-time-running").html(
                    zeroPad(hours, 2) +
                    ":" +
                    zeroPad(minutes, 2) +
                    ":" +
                    zeroPad(seconds.toFixed(3), 3)
                );
            } else {
                console.log("No voltages were retrieved.");
            }
        },
        error: function (request, status, error) {
            console.log("Error fetching voltage from API");
            console.table({
                error: error,
                status: status,
            });
        },
    });
};

let interval = setInterval(getData, 400);



function checkAlive() {
    let serverStatusModel = $("#serverDisconnectedModal");

    $.ajax({
        type: "get",
        url: "/api/alive",
        success: (data, text) => {
            if (!server_alive) {
                console.log("Server connection regained.");

                serverStatusModel.hide();
                server_alive = true;
                setInterval(getData, 400);
            }
        },
        error: (request, status, error) => {
            if (server_alive) {
                console.log("Server connection lost.");

                serverStatusModel.show();
                server_alive = false;
                clearInterval(getData, 400);

                let myModalEl = new Modal($("#serverDisconnectedModal"));
                myModalEl.show();
            }
        },
    });
}

$(() => {
    setInterval(async () => {
        checkAlive();
    }, 500);
    getDeviceInfo();
});

function initChart() {
    let datasets = []
    for (let channel in deviceConfig["channel_info"]) {
        datasets.push({
                label: deviceConfig["channel_info"][channel]["channel"],
                backgroundColor: "rgba(255, 255, 255, 0.1)",
                borderColor: `rgb(${Math.random()*255}, ${Math.random()*255}, ${Math.random()*255})`,
                data: []
            })
    }
    graph = new Chart($("#voltage-graph"), {
        // The type of chart we want to create
        type: "line",

        // The data for our dataset
        data: {
            datasets: datasets
        },

        // Configuration options go here
        options: {
            scales: {
                yAxes: [{
                    scaleLabel: {
                        display: true,
                        labelString: 'value'
                    },
                    ticks: {
                        suggestedMax: 0.05,
                        suggestedMin: -0.05,
                    },
                }],    
                xAxes: [{
                    type: "realtime",
                    realtime: {
                        duration: 1000,
                        refresh: 200,
                        
                        onRefresh: function (chart) {
                            chart.data.datasets.forEach(function (dataset) {
                                console.log("VP ASKJDNBASD", dataset);
                                for (let point of current_voltage_points[dataset["label"]]) {
                                  dataset.data.push(point);  
                                }
                                current_voltage_points[dataset["label"]] = []
                                console.log("Yesn't",dataset.data)
                            })
                            // chart.data.datasets.forEach(function(dataset) {

                            //   dataset.data.push({
                
                            //     x: Date.now(),
                
                            //     y: Math.random()
                
                            //   });
                
                            // });
                        }
                    }
                }],
            },
        },
        plugins: {
            streaming: {            // per-chart option
                frameRate: 30       // chart is drawn 30 times every second
            }
        }
    });
}

function getDeviceInfo() {
    $.ajax({
        type: "get",
        url: "/api/device-info",
        success: (data, text) => {
            deviceConfig = data;
            let virtualChannelCount = data["channel_info"]
                .map((e) => e["virt_channels"])
                .reduce((a, b) => a + b);
            let ChannelCount = data["channel_info"].length;

            $("#info-picoscope-type").html("PicoScope " + data["pico_scope_type"]);
            $("#info-channel-count").html(
                ChannelCount +
                " (" +
                data["channel_info"].map((a) => a["channel"]).join(" | ") +
                ")"
            );
            $("#info-virtual-channel-count").html(virtualChannelCount);

            $("#info-refresh-rate").html(
                data["refresh_rate"] +
                " / " +
                (data["refresh_rate"] * ChannelCount) / virtualChannelCount
            );
            $("#info-voltage-range").html(
                data["channel_info"]
                    .map((e) => e["channel"] + ": " + e["voltage_range"])
                    .join(", ")
            );
            // cubismInitialization();
            initChart();
        },
        error: (request, status, error) => {
            console.log("Error retrieving device data.");
            console.table({
                error: error,
                status: status,
            });
            getDeviceInfo();
        },
    });
}
