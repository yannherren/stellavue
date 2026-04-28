const socket = new WebSocket("/ws/tracker");

const main = document.querySelector(".main");
const logEl = document.querySelector('div[data-field="log"]');
const slider = document.querySelector(".slider");
const calibrating = document.querySelector(".calibrating");
const trackingButton = document.querySelector(".track-button");
const trackingContainer = document.querySelector(".tracking");
const controlsContainer = document.querySelector(".controls");
const upButton = document.querySelector(".up-button");
const downButton = document.querySelector(".down-button");
const percentageValue = document.querySelector(".percentage-value");
const percentageBar = document.querySelector(".percentage");
const shutterSpeed = document.querySelector(".shutter-speed");
const testCaptureButton = document.querySelector(".test-capture");
const autoCaptureButton = document.querySelector(".auto-capture");
const nightModeButton = document.querySelector(".night-mode");
const moveButtons = document.querySelectorAll(".move-button");
const stopButton = document.querySelector(".stop-button");

const adjustingSpeed = 6400;

let enabled = false;

const State = {
    IDLE: "IDLE",
    CALIBRATING: "CALIBRATING",
    MOVING: "MOVING",
    TRACKING: "TRACKING",
}

let state = State.IDLE
let autoCaptureOn = false;

const nightModeKey = "night_mode"
let nightMode = localStorage.getItem(nightModeKey) === 'true';
refreshNightMode()
nightModeButton.onclick = function () {
    nightMode = !nightMode;
    refreshNightMode();
    localStorage.setItem(nightModeKey, nightMode);
};
function refreshNightMode() {
    if (nightMode) {
        main.classList.add("theme-night");
        main.classList.remove("theme-default");
        nightModeButton.classList.add("selected");
    } else {
        main.classList.add("theme-default");
        main.classList.remove("theme-night");
        nightModeButton.classList.remove("selected")
    }
}

socket.addEventListener("open", (event) => {
    const command = 3; // 0b0011
    send_command(command);

    trackingButton.onclick = function () {
        let command = 2 + (state === State.TRACKING ? 0 : 1 << 4);
        send_command(command);
    }

    stopButton.onclick = function () {
        let command = 1;
        send_command(command);
    }

    slider.onchange = function () {
        const speed = this.value;
        const direction = 1;
        let command = 1 + (direction << 2) + (speed << 3);
        send_command(command);
    }

    upButton.onclick = function () {
        const upDirection = 1;
        let command = 1 + (upDirection << 4) + (adjustingSpeed << 5);
        send_command(command);
    }

    downButton.onclick = function () {
        const downDirection = 0;
        let command = 1 + (downDirection << 4) + (adjustingSpeed << 5);
        send_command(command);
    }

    testCaptureButton.onclick = function () {
        const command = 4; // 0b0100
        send_command(command);
    }

    autoCaptureButton.onclick = function () {
        let command;
        if (autoCaptureOn) {
            command = 5; // 0b0101
            command += shutterSpeed.value << 4
        } else {
            command = 7; // 0b0111
        }
        send_command(command);
        autoCaptureOn = !autoCaptureButton;
        autoCaptureButton.querySelector(".text").innerHTML = autoCaptureOn ? 'Stop auto capture' : 'Start auto capture'
    }
});

updateState(State.IDLE)

socket.addEventListener('message', async function (msg) {
    // const dataBytes = new Uint32Array(await msg.data.bytes());
    // console.log(dataBytes)

    // const data = dataBytes[3] << 24 + dataBytes[2] << 16 + dataBytes[1] << 8 + dataBytes[0]

    const buffer = await msg.data.arrayBuffer();
    const data = new DataView(buffer).getInt32(0)
    const commandType = data & 0xF;
    const payload = data >> 4;

    switch (commandType) {
        case 0:
            console.log("Stopped!")
            updateState(State.IDLE);
            break;
        case 0x1:
            const direction = data & 0x1;
            const speed = data >> 1;
            console.log("Constant movement started - dir: " + direction + ", speed: " + speed)
            updateState(State.MOVING);
            break;
        case 0x2:
            console.log("Tracking started")
            updateState(State.TRACKING);
            break;
        case 0x3:
            console.log("Tracker height changed: " + payload + "%")
            updatePercentage(payload)
            break;
        case 0x4:
            console.log("Calibration started")
            updateState(State.CALIBRATING);
            break;
        case 0xF:
            console.log("Status response", payload)
            updateState(State.CALIBRATING); //TODO: whatever
            break;
        default:
            console.log("Unknown command!")
    }

    const el = document.createElement('div');
    el.innerHTML = msg.data.toString();
    logEl.appendChild(el);
});

function updateState(newState) {
    state = newState;
    switch (state) {
        case State.CALIBRATING:
            calibrating.style.display = 'flex';
            trackingContainer.style.display = 'none';
            controlsContainer.style.display = 'none';
            break;
        case State.MOVING:
            calibrating.style.display = 'none';
            stopButton.style.display = 'flex';
            trackingContainer.style.display = 'inline-flex';
            controlsContainer.style.display = 'flex';
            moveButtons.forEach(it => it.style.display = 'none')
            trackingButton.disabled = true
            break;
        case State.IDLE:
            calibrating.style.display = 'none';
            trackingContainer.style.display = 'inline-flex';
            controlsContainer.style.display = 'flex';
            stopButton.style.display = 'none';
            trackingButton.querySelector(".text").innerHTML = "Start tracking"
            trackingButton.disabled = false
            moveButtons.forEach(it => {
                it.disabled = false;
                it.style.display = 'flex'
            })
            break;
        case State.TRACKING:
            calibrating.style.display = 'none';
            trackingContainer.style.display = 'inline-flex';
            controlsContainer.style.display = 'flex';
            trackingButton.querySelector(".text").innerHTML = "Stop tracking..."
            moveButtons.forEach(it => it.disabled = true)
            break;
    }
}

function updatePercentage(percentage) {
    percentageValue.innerHTML = percentage.toString() + "%";
    percentageBar.style.background = `conic-gradient(
            var(--theme-dark-primary) ${360 * (percentage / 100)}deg,
            var(--theme-dark-accent) ${360 * (percentage / 100)}deg
    )`
    console.log(`conic-gradient(
            var(--theme-dark-primary) ${360 * (percentage / 100)}deg,
            var(--theme-dark-accent) ${360 * (percentage / 100)}deg
    )`)
}

function send_command(command) {
    const buffer = new ArrayBuffer(4);
    const view = new DataView(buffer);
    view.setInt32(0, command, false);

    socket.send(buffer);
}