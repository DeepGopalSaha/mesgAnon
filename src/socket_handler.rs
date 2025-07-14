use futures::StreamExt;
use rmpv::Value;
use socketioxide::extract::*;
use tracing::{error, info, warn};

pub async fn handle_connect(socket: SocketRef, data: Value) {
    socket.emit("hello", "New user connected").ok();

    let mut user_room_id = String::new();

    // Parse payload data as a map
    if let Value::Map(map) = data {
        for (key, value) in map {
            if let Value::String(room_id) = key {
                if room_id.as_str().unwrap_or("") == "room_id" {
                    if let Value::String(val_str) = value {
                        user_room_id = val_str.as_str().unwrap_or("").to_string();

                        // socket.join is infallible — no need to match on Result
                        socket.join(user_room_id.clone());
                        info!("Socket {} joined room {}", socket.id, user_room_id);

                        // system_join emit IS fallible, handle it
                        if let Err(e) = socket
                            .to(user_room_id.clone())
                            .emit("system_join", "New user joined")
                            .await
                        {
                            warn!(
                                "Failed to emit join message to room {}: {}",
                                user_room_id, e
                            );
                        }
                    }
                }
            }
        }
    } else {
        warn!("Expected data to be a map, got {:?}", data);
    }

    // Handle message_recv with ack support
    let socket_copy = socket.clone();
    let user_room_id_clone = user_room_id.clone();

    socket_copy.on(
        "message_recv",
        async move |Data::<Value>(data)| match socket
            .to(user_room_id_clone.clone())
            .emit_with_ack::<_, String>("mesg_broadcast", &data)
            .await
        {
            Ok(ack_stream) => {
                ack_stream
                    .for_each(|(id, ack)| async move {
                        match ack {
                            Ok(ack_val) => {
                                info!("Ack received from socket {}: {:?}", id, ack_val);
                            }
                            Err(e) => {
                                warn!("Ack error from socket {}: {}", id, e);
                            }
                        }
                    })
                    .await;
            }
            Err(e) => {
                error!(
                    "Failed to emit message with ack to room {}: {}",
                    user_room_id_clone, e
                );
            }
        },
    );
}

pub async fn disconnect_socket(socket: SocketRef, data: Value) {
    if let Value::Map(map) = data {
        for (key, value) in map {
            if let Value::String(key_str) = key {
                if key_str.as_str().unwrap_or("") == "room_id" {
                    if let Value::String(val_str) = value {
                        let user_room_id = val_str.as_str().unwrap_or("").to_string();
                        match socket.within(user_room_id.clone()).disconnect().await {
                            Ok(_) => info!(
                                "Socket {} disconnected from room {}",
                                socket.id, user_room_id
                            ),
                            Err(e) => error!(
                                "Failed to disconnect socket {} from room {}: {}",
                                socket.id, user_room_id, e
                            ),
                        }
                    }
                }
            }
        }
    } else {
        warn!("Expected disconnect data to be a map, got {:?}", data);
    }
}
