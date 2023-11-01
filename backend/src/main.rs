use std::sync::Arc;
use std::time::Duration;

use axum::extract::Json as EJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum_typed_websockets::{Message, WebSocket, WebSocketUpgrade};
use db::{Item, List, ListDatabase};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::timeout;
use tower_http::timeout::TimeoutLayer;
use tower_http::{
    cors::CorsLayer, trace::TraceLayer, validate_request::ValidateRequestHeaderLayer,
};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

mod db;

struct AppState {
    db: Arc<ListDatabase>,
    tx: Sender<ServerMessage>,
    rx: Receiver<ServerMessage>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            tx: self.tx.clone(),
            rx: self.tx.subscribe(),
        }
    }
}

impl AppState {
    fn send_to_ws(&self, payload: ServerMessage) {
        let _ = self.tx.send(payload);
    }
}

#[derive(Debug, Error)]
pub enum ListError {
    #[error("Sql error")]
    SqlxError(#[from] sqlx::Error),
    #[error("No such list (id: {0})")]
    NoSuchList(i32),
    #[error("No such item (id: {0})")]
    NoSuchItem(i32),
    #[error("Can't create list with duplicate name '{0}'")]
    DuplicateListName(String),
}

type Result<T> = std::result::Result<T, ListError>;

impl IntoResponse for ListError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ListError::DuplicateListName(name) => (
                StatusCode::BAD_REQUEST,
                format!("Can't create list with duplicate name (name: '{name}')"),
            )
                .into_response(),
            ListError::NoSuchList(id) => (
                StatusCode::BAD_REQUEST,
                format!("Reference to list that doesn't exist (id: {id})"),
            )
                .into_response(),
            ListError::NoSuchItem(id) => (
                StatusCode::BAD_REQUEST,
                format!("Reference to item that doesn't exist (id: {id})"),
            )
                .into_response(),
            ListError::SqlxError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sqlx error: {e:?}"),
            )
                .into_response(),
        }
    }
}

lazy_static! {
    static ref USERNAME: &'static str = include_str!("../pass").split(":").next().unwrap();
    static ref PASSWORD: &'static str = include_str!("../pass").split(":").last().unwrap();
}

#[derive(Debug, Clone, Deserialize)]
struct ListCreate {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ListRename {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ItemCreate {
    list_id: i32,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ItemEdit {
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "tag", content = "value")]
enum ServerMessage {
    ListCreated(List),
    ItemCreated(Item),
    ListRenamed(List),
    ItemEdited(Item),
    ListRemoved(List),
    ItemRemoved(Item),
}

// There really aren't any message the client can send us
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "tag", content = "value")]
enum ClientMessage {}

async fn get_lists(State(s): State<AppState>) -> Result<Json<Vec<List>>> {
    Ok(Json(s.db.fetch_lists().await?))
}

async fn get_items(State(s): State<AppState>) -> Result<Json<Vec<Item>>> {
    Ok(Json(s.db.fetch_all_items().await?))
}

async fn get_list(Path(id): Path<i32>, State(s): State<AppState>) -> Result<Json<List>> {
    Ok(Json(s.db.fetch_list(id).await?))
}

async fn get_item(Path(id): Path<i32>, State(s): State<AppState>) -> Result<Json<Item>> {
    Ok(Json(s.db.fetch_item(id).await?))
}

async fn get_list_items(
    Path(list_id): Path<i32>,
    State(s): State<AppState>,
) -> Result<Json<Vec<Item>>> {
    Ok(Json(s.db.fetch_items(list_id).await?))
}

async fn post_lists(
    State(s): State<AppState>,
    EJson(payload): EJson<ListCreate>,
) -> Result<Json<List>> {
    let res = s.db.create_list(&payload.name).await?;
    s.send_to_ws(ServerMessage::ListCreated(res.clone()));
    Ok(Json(res))
}

async fn post_items(
    State(s): State<AppState>,
    EJson(payload): EJson<ItemCreate>,
) -> Result<Json<Item>> {
    let res = s.db.create_item(payload.list_id, &payload.content).await?;
    s.send_to_ws(ServerMessage::ItemCreated(res.clone()));
    Ok(Json(res))
}

async fn patch_list(
    Path(id): Path<i32>,
    State(s): State<AppState>,
    EJson(payload): EJson<ListRename>,
) -> Result<()> {
    let res = s.db.rename_list(id, &payload.name).await?;
    s.send_to_ws(ServerMessage::ListRenamed(res));
    Ok(())
}

async fn patch_item(
    Path(id): Path<i32>,
    State(s): State<AppState>,
    EJson(payload): EJson<ItemEdit>,
) -> Result<()> {
    let res = s.db.edit_item(id, &payload.content).await?;
    s.send_to_ws(ServerMessage::ItemEdited(res));
    Ok(())
}

async fn delete_list(Path(id): Path<i32>, State(s): State<AppState>) -> Result<()> {
    let res = s.db.remove_list(id).await?;
    s.send_to_ws(ServerMessage::ListRemoved(res));
    Ok(())
}

async fn delete_item(Path(id): Path<i32>, State(s): State<AppState>) -> Result<()> {
    let res = s.db.remove_item(id).await?;
    s.send_to_ws(ServerMessage::ItemRemoved(res));
    Ok(())
}

async fn ws_handler_upgrade(
    ws: WebSocketUpgrade<ServerMessage, ClientMessage>,
    State(s): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_handler(socket, s))
}

async fn ws_handler(mut socket: WebSocket<ServerMessage, ClientMessage>, mut s: AppState) {
    tracing::debug!("Got a websocket connection");
    'outer: loop {
        // Try to send a ping
        let Ok(_) = socket.send(Message::Ping(vec![])).await else {
            break;
        };

        // Expect a pong back within 4s
        let Ok(Some(Ok(Message::Pong(_)))) = timeout(Duration::from_secs(4), socket.recv()).await
        else {
            break;
        };

        // Get the updates in a loop, if no updates within 60s, loop back and send a ping, this makes sure that
        // no channels stays open for more than a minute if unused
        while let Ok(Ok(msg)) = timeout(Duration::from_secs(60), s.rx.recv()).await {
            if let Err(e) = socket.send(Message::Item(msg)).await {
                tracing::error!("Error when sending websocket message, closing connection: {e:?}");
                break 'outer;
            }
        }
    }
    tracing::debug!("Closed websocket connection because of an error or innactivity");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower-http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Arc::new(ListDatabase::new().await?);

    let (tx, rx) = tokio::sync::broadcast::channel(32);

    let app = Router::new()
        .route("/lists", get(get_lists).post(post_lists))
        .route("/items", get(get_items).post(post_items))
        .route(
            "/lists/:id",
            get(get_list).patch(patch_list).delete(delete_list),
        )
        .route("/lists/:id/items", get(get_list_items))
        .route(
            "/items/:id",
            get(get_item).patch(patch_item).delete(delete_item),
        )
        .layer(ValidateRequestHeaderLayer::basic(&USERNAME, &PASSWORD))
        .route("/ws", get(ws_handler_upgrade))
        .layer(TimeoutLayer::new(Duration::from_secs(4)))
        .layer(
            /*
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
                */
            CorsLayer::very_permissive(),
        )
        .with_state(AppState { db, tx, rx })
        .layer(TraceLayer::new_for_http());

    let listener = std::net::TcpListener::bind("0.0.0.0:9000").unwrap();

    axum::Server::from_tcp(listener)
        .unwrap()
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
