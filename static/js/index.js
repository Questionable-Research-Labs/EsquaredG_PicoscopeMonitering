import $ from 'jquery';
import { context } from 'cubism-es';


let current_voltage_points = []

$(function () {
    let interval = setInterval(async () => {
        let response = await $.get("api/data");
        current_voltage_points.push(response);
    }, 400);
});
