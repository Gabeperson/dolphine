use futures_util::{future, SinkExt, StreamExt, TryStreamExt};
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use rocket;
use rocket::get;
use rocket::http::ContentType;
use rocket::routes;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_repr::*;
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;

type Function = fn(String) -> Result<String, RustCallError>;
pub static WEBSERVER_PORT: usize = 8000;
static F: Dir = include_dir!("web");
pub static mut FILES: &Dir = &F;
pub static mut FILES_IN_USE: bool = false;
pub static WORKER_COUNT: usize = 1;
pub static HTTP_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub static SERVE_PATH: &'static str = "dolphine.js";
pub static WEBSOCKET_ADDR: &'static str = "127.0.0.1";
pub static WEBSOCKET_PORT: &'static str = stringify!(8080);
pub static FUNCTION_STORE: Lazy<Mutex<HashMap<String, (usize, Function)>>> =
    Lazy::new(|| Default::default());


#[macro_export]
macro_rules! async_function {
    ($func_name: ident) => {
        |input| {
            $crate::tokio::task::spawn(async move {
                $func_name(input).await
            }).join()
        }
    }
}


pub async fn block() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to wait for ctrl c");
}

pub fn set_files_directory(dir: &'static Dir) {
    unsafe {
        if FILES_IN_USE {
            panic!("Don't use the set_files function after you start the rocket server!");
        }
        FILES = dir;
    }
}

pub fn register_function<T>(name: T, function: Function, num_args: usize)
where
    T: ToString,
{
    let mut funcs = FUNCTION_STORE.lock().unwrap();
    funcs.insert(name.to_string(), (num_args, function));
}

#[derive(Error, Debug)]
pub enum RustCallError {
    #[error("data store disconnected")]
    Error(String),
}
pub use RustCallError::Error;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone)]
#[repr(u8)]
enum MessageType {
    ServerToClientResponse = 0,
    ClientToServerRequest = 1,
    // Register a rust function in javascript
    ServerToClientRegister = 2,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FunctionResponse {
    id: String,
    actiontype: MessageType,
    data: String,
    success: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FunctionRequest {
    id: String, // random n long unique identifier for the request to match with response default uses
    actiontype: MessageType, // type of action
    args: String, // arguments passed to function. Empty if called with no arguments
    function: String, // name of function. Used to get function from function map
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FunctionRegister {
    function: String,
    args: usize,
    actiontype: MessageType,
}

#[get("/")]
fn index() -> Option<(ContentType, String)> {
    unsafe {
        if let Some(file) = FILES.get_file("index.html") {
            if let Some(s) = file.contents_utf8() {
                return Some((ContentType::HTML, s.to_string()));
            }
        }
    }
    return None;
}

#[get("/<path>")]
fn get_file(path: &str) -> Option<(ContentType, String)> {
    if path == SERVE_PATH {
        return Some((
            ContentType::JavaScript,
            include_str!("..\\dolphine.js").to_string(),
        ));
    }
    unsafe {
        if let Some(file) = FILES.get_file(path) {
            if let Some(s) = file.contents_utf8() {
                if let Some(ext) = path.split(".").last() {
                    if let Some(content_type) = ContentType::from_extension(ext) {
                        return Some((content_type, s.to_string()));
                    }
                }
                return Some((ContentType::Text, s.to_string()));
            }
        }
    }
    return None;
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    let (mut write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    //let mut write = Arc::new(write);

    // put this into args later
    let functions = FUNCTION_STORE.lock().unwrap().clone();
    for (k, (arg_length, _ /* function */)) in functions.iter() {
        let d = FunctionRegister {
            args: *arg_length,
            function: k.to_owned(),
            actiontype: MessageType::ServerToClientRegister,
        };
        write
            .send(Message::Text(serde_json::to_string(&d).unwrap()))
            .await
            .unwrap();
    }

    _ = read
        .try_filter(|msg| future::ready(msg.is_text()))
        .fold(write, |mut write, message| async {
            let message = message.unwrap().into_text().unwrap();
            let json: FunctionRequest = serde_json::from_str(&message).unwrap();
            let func_name = json.clone().function;
            let func;
            let data;
            let success;
            {
                let functions = FUNCTION_STORE.lock().unwrap().clone();
                func = functions.get(&func_name).unwrap().clone();
                let d = async move { tokio::task::spawn_blocking(move || func.1(json.args)).await }
                    .await
                    .unwrap();
                data = match d {
                    Ok(i) => {
                        success = true;
                        i
                    }
                    Err(e) => {
                        success = false;
                        e.to_string()
                    }
                }
            }
            let response = FunctionResponse {
                id: json.id,
                actiontype: MessageType::ServerToClientResponse,
                data,
                success,
            };
            write
                .send(Message::Text(serde_json::to_string(&response).unwrap()))
                .await
                .unwrap();
            write
        })
        .await;
    println!("Connection closed.");
}

pub fn start_websocket_thread() {
    let _websocket_thread = tokio::task::spawn(async {
        let try_socket = TcpListener::bind(&format!("{}:{}", WEBSOCKET_ADDR, WEBSOCKET_PORT)).await;
        let listener = try_socket.expect("Failed to bind");
        println!(
            "Listening on: {}",
            format!("{}:{}", WEBSOCKET_ADDR, WEBSOCKET_PORT)
        );
        let threads = Arc::new(Mutex::new(Vec::new()));
        let threads_clone = threads.clone();
        let socket_thread = tokio::task::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                {
                    let mut threads = threads_clone.lock().unwrap();
                    threads.push(tokio::task::spawn(accept_connection(stream)));
                }
            }
        });
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl c");
        socket_thread.abort();
        let join_handles = threads.lock().unwrap();
        for join_handle in join_handles.iter() {
            join_handle.abort();
        }
        println!("Aborted all socket threads");
    });
}

pub fn start_rocket_thread() {
    unsafe {
        FILES_IN_USE = true;
    }
    let _rocket_thread = tokio::task::spawn(async {
        let figment = rocket::Config::figment()
            .merge(("address", HTTP_ADDR))
            .merge(("workers", WORKER_COUNT))
            .merge(("port", WEBSERVER_PORT));
        _ = rocket::custom(figment)
            .mount("/", routes![index, get_file])
            .launch()
            .await
            .unwrap();
    });
}
