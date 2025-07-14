let socket = null;
let selectedRoom = '';

function selectRoom(roomId) {
    if (socket) {
        socket.disconnect(); // Disconnect from previous room
    }

    selectedRoom = roomId;
    document.getElementById("messages").innerHTML = "";
    connectToSocket(roomId);
}

function connectToSocket(roomId) {
    socket = io("/", {
        auth: {
            room_id: roomId
        }
    });

    // System messages
    socket.on("hello", (msg) => {
        appendMessage(`System: ${msg}`, "system");
        triggerRoomCountUpdate(roomId); // <- ask server to emit update
    });

    // Incoming chat messages
    socket.on("mesg_broadcast", (msg) => {
        const isSystem = typeof msg === "string" && msg.startsWith("System:");
        appendMessage(msg, isSystem ? "system" : "other");

        // Send back ACK to server
        if (typeof ack === "function") {
            ack("Message received on client");
        }
    });

    // Update member count
    socket.on("update_count", (count) => {
        const roomName = formatRoomName(roomId);
        document.getElementById("chatHeader").textContent = `Room: ${roomName} (${count} online)`;
    });

    // System join message from backend
    socket.on("system_join", (msg) => {
        appendMessage(`System: ${msg}`, "system");
    });


    // Immediately fetch current count (fallback in case socket event lags)
    updateRoomHeader(roomId);

    // Trigger count update when disconnected
    socket.on("disconnect", () => {
        triggerRoomCountUpdate(roomId);
    });
}

// Ask backend to emit `update_count`
function triggerRoomCountUpdate(roomId) {
    fetch(`/room/${roomId}/users`).catch(err => {
        console.error("Failed to trigger count update:", err);
    });
}

// Fallback UI fetch for room header
function updateRoomHeader(roomId) {
    fetch(`/room/${roomId}/users`)
        .then(res => res.json())
        .then(data => {
            const roomName = formatRoomName(roomId);
            const count = data.room_count || 0;
            document.getElementById("chatHeader").textContent = `Room: ${roomName} (${count} online)`;
        })
        .catch(err => {
            console.error("Error fetching member count:", err);
            document.getElementById("chatHeader").textContent = `Room: ${formatRoomName(roomId)}`;
        });
}

function formatRoomName(roomId) {
    const name = roomId.split('-')[1] || roomId;
    return name.charAt(0).toUpperCase() + name.slice(1);
}

function sendMessage() {
    const input = document.getElementById("chatInput");
    const text = input.value.trim();
    if (!text || !socket) return;

    socket.emit("message_recv", text);
    appendMessage(text, "you");
    input.value = "";
}

function appendMessage(msg, type) {
    const messages = document.getElementById("messages");
    const div = document.createElement("div");
    div.className = "message";

    if (type === "you") {
        div.classList.add("you");
    } else if (type === "system") {
        div.classList.add("system");
    } else {
        div.classList.add("other");
    }

    div.textContent = msg;
    messages.appendChild(div);
    messages.scrollTop = messages.scrollHeight;
}

window.addEventListener("DOMContentLoaded", () => {
    const input = document.getElementById("chatInput");
    input.addEventListener("keydown", function (event) {
        if (event.key === "Enter") {
            event.preventDefault();
            sendMessage();
        }
    });
});



function toggleSidebar() {
    document.getElementById("sidebarWrapper").classList.toggle("active");
}

function selectRoomAndClose(roomId) {
    selectRoom(roomId);
    const wrapper = document.getElementById("sidebarWrapper");
    wrapper.classList.remove("active");
}



    function updateSidebarHeading() {
        const sidebarHeading = document.querySelector(".sidebar h2");
        if (window.innerWidth <= 1000) {
            sidebarHeading.textContent = "Rooms"; // Hide on small screens
        } else {
            sidebarHeading.textContent = "Rooms"; // Restore on large screens
        }
    }

    window.addEventListener("resize", updateSidebarHeading);
    window.addEventListener("DOMContentLoaded", updateSidebarHeading);

