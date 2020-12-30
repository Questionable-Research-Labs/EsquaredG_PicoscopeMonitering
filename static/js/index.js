import $ from 'jquery';
import { context } from 'cubism-es';


let current_voltage_points = []
let server_alive = true;

$(function () {
    setInterval(async () => {
        if (server_alive) {
            let response = await $.get("api/data");
            current_voltage_points.push(response);
        }
    }, 400);

    setInterval(async () => {
        checkAlive();
    }, 500)
});

function checkAlive() {
    let serverStatusModel = $("#serverDisconnectedModal");
        $.ajax({
            type: "get", url: "/api/alive",
            success: function (data, text) {
                if (!server_alive) {
                    console.log("Server connection regained.")
                    serverStatusModel.hide();
                    server_alive = true
                }
                
            },
            error: function (request, status, error) {
                if (server_alive) {
                    console.log("Server connection lost.")
                    serverStatusModel.show();
                    server_alive = false;
                }
            }
        });
}