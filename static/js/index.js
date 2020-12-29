import $ from 'jquery';

$(document).ready(() => {
    let interval = setInterval(async () => {
        let response = await $.get("api/data");
        // TODO: Do shit with the data, for now it can just vibe
    }, 200);
    
});
