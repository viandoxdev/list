use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use base64::{prelude::BASE64_URL_SAFE, Engine};
use db::{Item, List, ListDatabase};
use futures::StreamExt;
use hyper::{
    body::{HttpBody, self},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

mod db;

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
    #[error("Serialization/Deserialization error")]
    SerdeError(#[from] serde_json::Error),
    #[error("Body error")]
    HyperError(#[from] hyper::Error)
}

lazy_static! {
    static ref LISTS: Regex = Regex::new(r"^/lists/*$").unwrap();
    static ref ITEMS: Regex = Regex::new(r"^/items/*$").unwrap();
    static ref LIST: Regex = Regex::new(r"^/lists/([0-9])+/*$").unwrap();
    static ref ITEM: Regex = Regex::new(r"^/items/([0-9])+/*$").unwrap();
    static ref LIST_ITEMS: Regex = Regex::new(r"^/lists/([0-9]+)/items/*$").unwrap();
    static ref AUTH: String = format!("Basic {}", BASE64_URL_SAFE.encode(include_str!("../pass")));
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

async fn handle_req(
    db: Arc<ListDatabase>,
    req: Request<Body>,
    res: &mut Response<Body>,
) -> Result<(), ListError> {
    log::debug!("Got request: {:?}", req);
    // Check auth header
    match req.headers().get("Authorization") {
        Some(auth) => {
            let str = auth.to_str().unwrap_or_default();

            if str != &*AUTH {
                log::warn!("Request failed authentication");
                // Auth failed, set status and exit
                *res.status_mut() = StatusCode::UNAUTHORIZED;
                return Ok(());
            }
        }
        _ => {
            log::debug!("Request missing authentication");
            // Auth missing, set status and exit
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut()
                .insert("WWW-Authenticate", "Basic".parse().unwrap());
            return Ok(());
        }
    }

    let path = req.uri().path();
    // Match against all the regexes at once
    let regexes: [&Regex; 5] = [&*LISTS, &*LIST_ITEMS, &*ITEMS, &*LIST, &*ITEM];
    let [c_lists, c_list_items, c_items, c_list, c_item] = regexes.map(|re| re.captures(path));

    // Set here so that anything failing can go strait to return.
    *res.status_mut() = StatusCode::NOT_FOUND;
    let method = req.method().clone();

    if let Some(_) = c_lists {
        // /lists
        match method {
            Method::GET => {
                // Get all lists
                let lists = db.fetch_lists().await?;
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&lists)?.into();
                Ok(())
            }
            Method::POST => {
                // Create a list
                let bytes = body::to_bytes(req.into_body()).await?;
                let create_info: ListCreate = serde_json::from_slice(&bytes)?;
                let list = db.create_list(&create_info.name).await?;
                // Return the created list
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&list)?.into();
                Ok(())
            }
            _ => return Ok(()),
        }
    } else if let Some(_) = c_items {
        // /items
        match method {
            Method::GET => {
                // Get all items
                let items = db.fetch_all_items().await?;
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&items)?.into();
                Ok(())
            }
            Method::POST => {
                // Create an item
                let bytes = body::to_bytes(req.into_body()).await?;
                let create_info: ItemCreate = serde_json::from_slice(&bytes)?;
                let item = db.create_item(create_info.list_id, &create_info.content).await?;
                // Return the created item
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&item)?.into();
                Ok(())
            }
            _ => return Ok(()),
        }
    } else if let Some(captures) = c_list {
        let list_id: i32 = captures[1].parse().unwrap_or_default();
        // /lists/<list_id>
        match method {
            Method::GET => {
                // Get a specific list
                let list = db.fetch_list(list_id).await?;
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&list)?.into();
                Ok(())
            }
            Method::PATCH => {
                // Rename a list
                let bytes = body::to_bytes(req.into_body()).await?;
                let rename: ListRename = serde_json::from_slice(&bytes)?;
                db.rename_list(list_id, &rename.name).await?;
                *res.status_mut() = StatusCode::OK;
                Ok(())
            }
            Method::DELETE => {
                // Delete a list
                db.remove_list(list_id).await?;
                *res.status_mut() = StatusCode::OK;
                Ok(())
            }
            _ => return Ok(()),
        }
    } else if let Some(captures) = c_item {
        let item_id: i32 = captures[1].parse().unwrap_or_default();
        // /items/<item_id>
        match method {
            Method::GET => {
                // Get a specific item
                let item = db.fetch_item(item_id).await?;
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&item)?.into();
                Ok(())
            }
            Method::PATCH => {
                // Edit an item
                let bytes = body::to_bytes(req.into_body()).await?;
                let edit: ItemEdit = serde_json::from_slice(&bytes)?;
                db.edit_item(item_id, &edit.content).await?;
                *res.status_mut() = StatusCode::OK;
                Ok(())
            }
            Method::DELETE => {
                // Delete an item
                db.remove_item(item_id).await?;
                *res.status_mut() = StatusCode::OK;
                Ok(())
            }
            _ => return Ok(()),
        }
    } else if let Some(captures) = c_list_items {
        let list_id: i32 = captures[1].parse().unwrap_or_default();
        // /lists/<list_id>/items
        match method {
            Method::GET => {
                // Get the items of a list
                let items = db.fetch_items(list_id).await?;
                *res.status_mut() = StatusCode::OK;
                *res.body_mut() = serde_json::to_string(&items)?.into();
                Ok(())
            }
            _ => return Ok(()),
        }
    } else {
        // if nothing matches, we go straight to return with a 404
        Ok(())
    }
}

async fn handle_req_wrap(
    db: Arc<ListDatabase>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let mut res = Response::new(Body::empty());

    match handle_req(db, req, &mut res).await {
        Ok(()) => Ok(res),
        Err(ListError::NoSuchList(id)) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            *res.body_mut() = format!("Reference to a list that doesn't exist (id: {id})").into();
            Ok(res)
        }
        Err(ListError::NoSuchItem(id)) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            *res.body_mut() = format!("Reference to an item that doesn't exist (id: {id})").into();
            Ok(res)
        }
        Err(ListError::DuplicateListName(name)) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            *res.body_mut() = format!("Trying to create a list with a duplicate name (name: '{name}')").into();
            Ok(res)
        }
        Err(ListError::HyperError(e)) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            *res.body_mut() = format!("Error with request body: {e}").into();
            Ok(res)
        }
        Err(ListError::SerdeError(e)) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            *res.body_mut() = format!("Error with request body: {e}").into();
            Ok(res)
        }
        Err(e) => {
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *res.body_mut() = format!("Server error: {e:?}").into();
            Ok(res)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), ListError> {
    env_logger::init();

    let db = Arc::new(ListDatabase::new().await?);
    log::info!("Database initialized");
    let addr = SocketAddr::from(([0, 0, 0, 0], 9000));

    let make_svc = make_service_fn(move |_conn| {
        let db = db.clone();
        log::info!("Received connection");
        async move { Ok::<_, Infallible>(service_fn(move |r| handle_req_wrap(db.clone(), r))) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    log::info!("Server started!");

    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}
