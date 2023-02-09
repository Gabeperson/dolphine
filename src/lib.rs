use browsers::open_browser;
use futures_util::{future, SinkExt, StreamExt, TryStreamExt};
pub use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use rocket;
use rocket::get;
use rocket::http::ContentType;
pub use rocket::main;
use rocket::routes;
use rocket::State;
use serde::{Deserialize, Serialize};
pub use serde_json;
use serde_repr::*;
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
pub use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;
mod browsers;
pub use browsers::Browser;
//extern crate macros;
pub use eyre::Report;
pub use macros::async_function;
pub use macros::function;

type Function = fn(String) -> Result<String, Report>;

static DIRS: Lazy<RwLock<HashMap<usize, &Dir>>> = Lazy::new(|| Default::default());

static DIRS_COUNTER: Lazy<RwLock<usize>> = Lazy::new(|| RwLock::new(0));

#[derive(Clone, Debug)]
pub struct Dolphine {
    pub webserver_port: u16,
    pub worker_count: usize,
    pub http_addr: IpAddr,
    pub serve_path: String,
    pub websocket_addr: IpAddr,
    source_dynamic: String,
    pub websocket_port: u16,
    function_store: HashMap<String, (usize, Function)>,
    source: TypeSource,
    id: usize,
}

/*
TODO:
Hide cmd window
Exit handler
ping pong keep alive




*/

#[derive(Clone, Debug)]
enum TypeSource {
    LocalFolder,
    StaticFolder,
}

impl Dolphine {
    pub fn new() -> Dolphine {
        let mut id_gen = DIRS_COUNTER.write().unwrap();
        let id = *id_gen;
        *id_gen = *id_gen + 1;
        drop(id_gen);

        Dolphine {
            webserver_port: 8000,
            worker_count: 1,
            http_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            serve_path: String::from("dolphine.js"),
            websocket_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            websocket_port: 8080,
            function_store: HashMap::new(),
            source_dynamic: String::new(),
            source: TypeSource::StaticFolder,
            id,
        }
    }

    pub fn open_page(&self, b: Browser) {
        open_browser(
            b,
            format!("{}:{}", self.http_addr.to_string(), self.webserver_port),
        );
    }

    pub async fn block(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for ctrl c");
    }

    pub fn set_local_file_directory<S>(&mut self, dir: S)
    where
        S: ToString,
    {
        self.source = TypeSource::LocalFolder;
        let mut dirs = DIRS.write().unwrap();
        dirs.remove(&self.id);
        drop(dirs);
        self.source_dynamic = dir.to_string();
    }

    pub fn set_static_file_directory(&mut self, dir: &'static Dir) {
        self.source = TypeSource::StaticFolder;
        self.source_dynamic = String::new();
        let mut dirs = DIRS.write().unwrap();
        dirs.insert(self.id, dir);
    }

    pub async fn init(&self, opt_block: bool) {
        self.start_rocket_thread();
        self.start_websocket_thread();
        if opt_block {
            self.block().await;
        }
    }

    pub fn register_function<T>(&mut self, name: T, function: Function, num_args: usize)
    where
        T: ToString,
    {
        // add function to the hashmap
        self.function_store
            .insert(name.to_string(), (num_args, function));
    }

    pub fn start_websocket_thread(&self) {
        let s = self.clone();
        let functions = self.function_store.clone();
        let _websocket_thread = tokio::task::spawn(async move {
            let try_socket = TcpListener::bind(&format!(
                "{}:{}",
                s.websocket_addr.to_string(),
                s.websocket_port.to_string()
            ))
            .await;
            let listener = try_socket.expect("Failed to bind");
            println!(
                "Listening on: {}",
                format!("{}:{}", s.websocket_addr.to_string(), s.webserver_port)
            );
            let threads = Arc::new(Mutex::new(Vec::new()));
            let threads_clone = threads.clone();
            let socket_thread = tokio::task::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    {
                        let mut threads = threads_clone.lock().unwrap();
                        threads.push(tokio::task::spawn(accept_connection(
                            stream,
                            functions.clone(),
                        )));
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

    pub fn start_rocket_thread(&self) {
        let serve_path = self.serve_path.clone();
        let source = self.source.clone();
        let source_dynamic = self.source_dynamic.clone();
        let http_addr = self.http_addr.clone();
        let worker_count = self.worker_count.clone();
        let webserver_port = self.webserver_port.clone();
        let id = self.id;
        let wp = "\"".to_string()
            + &self.websocket_addr.to_string()[..]
            + ":"
            + &self.websocket_port.to_string()[..]
            + "\"";

        let _rocket_thread = tokio::task::spawn(async move {
            let figment = rocket::Config::figment()
                .merge(("address", http_addr))
                .merge(("workers", worker_count))
                .merge(("port", webserver_port));
            let mut _rocket = rocket::custom(figment).mount("/", routes![index, get_file]);

            match source {
                TypeSource::StaticFolder => {
                    _rocket = _rocket.manage(StateManager::new(
                        Source::Static(id), /*Source::Static((s.source_static).clone())*/
                        serve_path,
                        wp,
                    ));
                }
                TypeSource::LocalFolder => {
                    _rocket = _rocket.manage(StateManager::new(
                        Source::Local(source_dynamic),
                        serve_path,
                        wp,
                    ));
                }
            }
            let _rocket = _rocket.launch().await.unwrap();
        });
    }
}

#[derive(Debug, Clone)]
enum Source {
    Static(usize),
    Local(String),
}

#[derive(Debug, Clone)]
struct StateManager {
    source: Source,
    serve_path: String,
    websocket_path: String,
}

impl StateManager {
    fn new(s: Source, s1: String, wp: String) -> StateManager {
        StateManager {
            source: s,
            serve_path: s1,
            websocket_path: wp,
        }
    }
}

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
fn index(statemanager: &State<StateManager>) -> Option<(ContentType, String)> {
    match &statemanager.source {
        Source::Static(f) => {
            let file_hashmap = DIRS.read().unwrap();
            if let Some(f) = file_hashmap.get(f) {
                if let Some(file) = f.get_file("index.html") {
                    if let Some(s) = file.contents_utf8() {
                        return Some((ContentType::HTML, s.to_string()));
                    }
                }
            }
        }
        Source::Local(f) => {
            let f = fs::read_to_string(f);
            if let Ok(s) = f {
                return Some((ContentType::HTML, s.to_string()));
            }
        }
    }
    return None;
}

#[get("/<path>")]
fn get_file(path: &str, statemanager: &State<StateManager>) -> Option<(ContentType, Vec<u8>)> {
    if path == statemanager.serve_path {
        let mut resp = include_bytes!("..\\dolphine.js").to_vec();
        let js_code = format!(
            "\ndolphine._socket = {};\ndolphine._init();",
            statemanager.websocket_path
        );
        let path = js_code.as_bytes();
        resp.extend_from_slice(path);
        return Some((ContentType::JavaScript, resp));
    }
    match &statemanager.source {
        Source::Static(f) => {
            let file_hashmap = DIRS.read().unwrap();
            if let Some(f) = file_hashmap.get(f) {
                if let Some(file) = f.get_file("index.html") {
                    if let Some(ext) = path.split(".").last() {
                        let s = file.contents();
                        if let Some(content_type) = ContentType::from_extension(ext) {
                            return Some((content_type, s.to_vec()));
                        }
                        return Some((ContentType::Text, s.to_vec()));
                    }
                }
            }
        }
        Source::Local(f) => {
            let f = fs::read(f);
            if let Ok(file) = f {
                if let Some(ext) = path.split(".").last() {
                    let s = file;
                    if let Some(content_type) = ContentType::from_extension(ext) {
                        return Some((content_type, s));
                    }
                    return Some((ContentType::Text, s));
                }
                return Some((ContentType::Text, file));
            }
        }
    }
    return None;
}

async fn accept_connection(stream: TcpStream, functions: HashMap<String, (usize, Function)>) {
    //let addr = stream
    //.peer_addr()
    //.expect("connected streams should have a peer address");
    //println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    //println!("New WebSocket connection: {}", addr);

    let (mut write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    //let mut write = Arc::new(write);

    // put this into args later

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
                        format!("{:#}", e)
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
