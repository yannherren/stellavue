const socket = new WebSocket("/ws/tracker");
const slider = document.querySelector(".slider");

socket.addEventListener("open", (event) => {
    slider.onchange = function() {
        const speed = this.value;
        const direction = 1;
        let command = 1 + (direction << 2) + (speed << 3);

        const buffer = new ArrayBuffer(2);
        const view = new DataView(buffer);
        view.setInt16(0, command, false);

        socket.send(buffer);
    }
});