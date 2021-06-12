import $ from "jquery";
import { Modal } from "bootstrap";
import {  } from "@popperjs/core";
import { Chart } from "chart.js";
import 'chartjs-plugin-streaming';
import "../css/main.scss";

let data = {};
let graph;

let current_voltage_points = {};
let refreshrate_testing_waitlist = {};
let server_alive = true;
let deviceConfig = {};

let chart_initalised = false;

// Absolutely needed
const isEven = (a) => a % 2 == 0;
const zeroPad = (num, places) => String(num).padStart(places, "0");
const average = (array) => array.reduce((a, b) => a + b) / array.length;


const getData = async () => {
    $.ajax({
        type: "get",
        url: "/api/data",
        success: function (data, text) {
            let voltages = data["voltages"];
            console.log(voltages);
            /*
            voltages = {"channel letter": [[volt,time since server start, time of record],]}
            */
            console.log("No.",voltages);
            if (Object.keys(voltages).length != 0) {
                for (let channel of Object.keys(voltages)) {
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
                }

                let top_channel = voltages[Object.keys(voltages)[0]];

                let ms = top_channel[top_channel.length - 1][1];

                let seconds = ms / 1000;
                let hours = parseInt(seconds / 3600); // 3,600 seconds in 1 hour
                seconds = seconds % 3600; // seconds remaining after extracting hours
                let minutes = parseInt(seconds / 60); // 60 seconds in 1 minute
                seconds = seconds % 60;

                $("#info-last-report").html(
                    zeroPad(hours, 2) +
                    ":" +
                    zeroPad(minutes, 2) +
                    ":" +
                    zeroPad(seconds.toFixed(3), 3)
                );
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





function checkAlive() {
    

    $.ajax({
        type: "get",
        url: "/api/alive",
        success: (data, text) => {
            if (!server_alive) {
                console.log("Server connection regained.");

                $("#serverDisconnectedModal").modal({show: false});
                server_alive = true;
                setInterval(getData, 400);
            }
        },
        error: (request, status, error) => {
            console.log("Server not alive", server_alive)
            if (server_alive) {

                console.log("Server connection lost.");
                $("#serverDisconnectedModal").modal({show: true});
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
        getData();
    }, 400);
    setInterval(async () => {
        checkAlive();
    }, 500);
    setInterval(async () => {
        getDeviceInfo();
    }, 1000);
    
    
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
                                console.log(current_voltage_points);
                                if (current_voltage_points[dataset["label"]] !== undefined) {
                                    for (let point of current_voltage_points[dataset["label"]]) {
                                        dataset.data.push(point);  
                                      }
                                      current_voltage_points[dataset["label"]] = []
                                }

                            })
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

/// Caculates the refresh rate of the data
function testRefreshRate() {
    console.log()
    if (current_voltage_points === {}) {
        return "-"
    }
    let refresh_rates = []
    for (let refresh_channel in current_voltage_points) {
        let voltage_points = current_voltage_points[refresh_channel];

        // Sometimes there is only one data point, so we add it the
        // refreshrate_testing_waitlist so we can caculate it next time.
        if (refreshrate_testing_waitlist[refresh_channel] !== undefined) {
            voltage_points.push(...refreshrate_testing_waitlist[refresh_channel]);
        }
        // Clear the waitlist for this channel
        delete refreshrate_testing_waitlist[refresh_channel];
        
        console.log(voltage_points);
        
        // After the waitlist, again we cannot calculate the refresh rate
        // with only one data point, so we just return the last value
        if (voltage_points.length <= 1) {
            return $("#info-recived-refresh-rate").html();
        }

        console.log("Wow",voltage_points)
        
        // 
        let avg_diff = Math.abs(
            Date.parse(voltage_points[0]["x"]) -
            Date.parse(voltage_points[voltage_points.length-1]["x"])
        ) / (voltage_points.length - 1);

        let hz = Math.round((1/(avg_diff/1000))*100)/100;
        console.log("Avg Diff",hz);
        refresh_rates.push(hz);
    }
    if (refresh_rates.length === 0) {
        return "-";
    } else {
        return average(refresh_rates)
    }
    

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

            $("#info-target-refresh-rate").html(
                data["refresh_rate"] +
                " / " +
                (data["refresh_rate"] * ChannelCount) / virtualChannelCount
            );
            $("#info-recived-refresh-rate").html(testRefreshRate());
            $("#info-voltage-range").html(
                data["channel_info"]
                    .map((e) => e["channel"] + ": " + e["voltage_range"])
                    .join(", ")
            );
            if (!chart_initalised) {
                initChart();
                chart_initalised = true;
            }
            
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
