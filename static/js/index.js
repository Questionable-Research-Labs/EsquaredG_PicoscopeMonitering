import $ from "jquery";
// import {context} from "cubism-es";
import {Modal} from 'bootstrap';

const getData = async () => {
    $.ajax({
        type: "get", url: "/api/data",
        success: function (data, text) {
            current_voltage_points.push(JSON.parse(data));

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

let deviceInfo = {};

let current_voltage_points = []
let server_alive = true;

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

$(() => {
    // setInterval(async () => {
    //     checkAlive();
    // }, 500);
    $.ajax({
        type: "get", url: "/api/device-info",
        success: (data, text) => {
            deviceInfo = JSON.parse(data);
            $("#info-picoscope-type").html("PicoScope " + deviceInfo["pico_scope_type"]);
            $("#info-channel-count").html(deviceInfo["channel_info"].length);
            $("#info-vertual-channel-count").html(deviceInfo["channel_info"].map((e) => e["virt_channels"]).reduce((a,b) => a + b));
            $("#info-refresh-rate").html(deviceInfo["refresh_rate"]);
            $("#info-voltage-range").html(deviceInfo["channel_info"][0]["voltage_range"])
        },
        error: (request, status, error) => {
            console.log("Error retrieveing device data.");
            console.table({
                "error": error,
                "status": status
            })
        }
    })
});