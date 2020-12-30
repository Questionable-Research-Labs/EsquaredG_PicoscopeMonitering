import $ from "jquery";
import {context} from "cubism-es";

const getData = async () => {
    try {
        let response = await $.get("api/data");
        current_voltage_points.push(response);
    } catch (e) {
        console.log("Error fetching data from API");
    }
};

let current_voltage_points = []
let server_alive = true;

let interval = setInterval(getData, 400);

setInterval(async () => {
    checkAlive();
}, 500);

function checkAlive() {
    $.ajax({
        type: "get", url: "/api/alive",
        success: function (data, text) {
            if (!server_alive) {
                console.log("Server connection regained.")
                serverStatusModel.hide();
                server_alive = true;
                setInterval(getData, 400);
            }

        },
        error: function (request, status, error) {
            if (server_alive) {
                console.log("Server connection lost.")
                serverStatusModel.show();
                server_alive = false;
                clearInterval(interval);
            }
        }
    });
    let serverStatusModel = $("#serverDisconnectedModal");
}