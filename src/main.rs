use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::{routing::get, Router};
pub mod socket_handler;
use rmpv::Value;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};

use dotenv::dotenv;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

use tracing_subscriber::EnvFilter;

use std::sync::Arc;
use tera::{Context, Tera};

#[derive(Clone)]
struct Appstate {
    io: SocketIo,
    tera: Arc<Tera>,
}
async fn home(State(state): State<Arc<Appstate>>) -> impl IntoResponse {
    let ctx = Context::new();
    match state.tera.render("index.html", &ctx) {
        Ok(rendered) => Html(rendered).into_response(),
        Err(err) => Html(state.tera.render("501.html", &ctx).unwrap_or_else(|e| {
            error!("Cannot render html page due to {}", e);
            panic!(
                "Cannot render page due to internal server error due to {} and {}",
                err, e
            )
        }))
        .into_response(),
    }
}

async fn fallback_page(State(state): State<Arc<Appstate>>) -> impl IntoResponse {
    info!("Coming to fallback page");
    let ctx = Context::new();
    match state.tera.render("404.html", &ctx) {
        Ok(rendered) => Html(rendered).into_response(),
        Err(err) => Html(state.tera.render("501.html", &ctx).unwrap_or_else(|e| {
            panic!(
                "Cannot render page due to internal server error due to {} and {}",
                err, e
            )
        }))
        .into_response(),
    }
}

#[derive(Serialize, Deserialize)]
struct RoomCount {
    room_id_val: String,
    room_count: usize,
}

async fn active_members(
    State(state): State<Arc<Appstate>>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    info!("Accessed /room/{}/members", &room_id);
    let room_clone = room_id.clone();
    let state_io_clone = state.io.clone();
    let sockets = state.io.within(room_id).sockets();
    let number_connections = sockets.len();

    let members = if number_connections == 0 {
        1
    } else {
        number_connections
    };

    let _ = state_io_clone
        .to(room_clone.clone())
        .emit("update_count", &members)
        .await
        .ok();

    let response = Json(RoomCount {
        room_id_val: room_clone,
        room_count: number_connections,
    });

    (StatusCode::OK, response)
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        // This allows you to use, e.g., `RUST_LOG=info` or `RUST_LOG=debug`
        // when running the app to set log levels.
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .init();

    let socket_ip = std::env::var("IP").unwrap_or_else(|_| "0.0.0.0".to_string());

    let socket_port = std::env::var("PORT").unwrap_or_else(|_| "10000".to_string());

    let template_path = std::env::var("TEMPLATE_PATH").unwrap_or_else(|e| {
        warn!("Cannot find template_path due to {}", e);
        panic!("Cannot find TEMPLATE_PATH in env file");
    });

    let static_path = std::env::var("STATIC_PATH").unwrap_or_else(|e| {
        warn!("Cannot find static path due to {}", e);
        panic!("Cannot find STATIC_PATH in env file");
    });

    let tera_path = match Tera::new(&template_path) {
        Ok(t) => Arc::new(t),
        Err(e) => {
            warn!("Cannot find tera path due to  {}", e);
            panic!("Parsing error(s): {}", e);
        }
    };

    let socket_addr = format!("{}:{}", socket_ip, socket_port);
    let sock = socket_addr.clone();

    let (layer, io) = SocketIo::new_layer();

    let state = Arc::new(Appstate {
        io: io.clone(),
        tera: tera_path,
    });

    io.ns(
        "/",
        move |socket: SocketRef, Data(data): Data<Value>| async move {
            socket_handler::handle_connect(socket, data).await;
        },
    );

    io.ns(
        "/disconnect",
        move |socket: SocketRef, Data(data): Data<Value>| async move {
            socket_handler::disconnect_socket(socket, data).await;
        },
    );

    let static_files = ServeDir::new(static_path);

    let app = Router::new()
        .nest_service("/static", static_files)
        .route("/", get(home))
        .route("/room/{room_id}/users", get(active_members))
        .fallback(fallback_page)
        .with_state(state)
        .layer(layer);

    let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();

    info!("Server starting at {}", sock);
    axum::serve(listener, app).await.unwrap()
}
