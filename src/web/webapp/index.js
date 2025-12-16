const socket = new WebSocket("/ws/tracker");

const logEl = document.querySelector('div[data-field="log"]');
const slider = document.querySelector(".slider");
const trackingButton = document.querySelector(".track-button");
const upButton = document.querySelector(".up-button");
const downButton = document.querySelector(".down-button");

const adjustingSpeed = 6400;

let enabled = false;
let adjusting = false;

socket.addEventListener("open", (event) => {
    trackingButton.onclick = function () {
        let command = 2 + (enabled ? 0 : 1 << 2);
        send_command(command);
    }
    slider.onchange = function() {
        const speed = this.value;
        const direction = 1;
        let command = 1 + (direction << 2) + (speed << 3);
        send_command(command);
    }

    upButton.onclick = function () {
        let command;
        const upDirection = 1;
        if (adjusting) command = 1;
        else command = 1 + (upDirection << 2) + (adjustingSpeed << 3);
        adjusting = !adjusting;
        send_command(command);
    }

    downButton.onclick = function () {
        let command;
        const downDirection = 0;
        if (adjusting) command = 1;
        else command = 1 + (downDirection << 2) + (adjustingSpeed << 3);
        adjusting = !adjusting;
        send_command(command);
    }
});

socket.addEventListener('message', function (msg) {
    const el = document.createElement('div');
    el.innerHTML = msg.data.toString();
    logEl.appendChild(el);
});

function send_command(command) {
    const buffer = new ArrayBuffer(2);
    const view = new DataView(buffer);
    view.setInt16(0, command, false);

    socket.send(buffer);
}