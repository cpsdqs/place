extern crate ws;
#[macro_use]
extern crate serde_derive;
extern crate base64;
extern crate crypto_hash;
extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, Weak};
use std::{env, fs, io, thread, time};

mod canvas;
mod console;
mod login;
mod messages;

use canvas::Canvas;
use login::Logins;
use messages::{ClientMessage, ClientRequest};

fn main() {
    let (update_tx, update_rx) = mpsc::channel();
    let global = Arc::new(Mutex::new(GlobalState::new()));
    let global_weak = Arc::downgrade(&global);

    let update_thread_global = Arc::clone(&global);
    thread::spawn(move || update_thread(update_rx, update_thread_global));

    let mut conn_id_counter = 0;

    ws::listen("127.0.0.1:8000", |out| {
        conn_id_counter += 1;
        ConnHandler::new(global_weak.clone(), out, update_tx.clone(), conn_id_counter)
    }).unwrap();
}

const MAX_PIXELS_PER_FRAME: usize = 3000;

pub struct ClientSender {
    id: u64,
    id_info: String,
    out: Arc<ws::Sender>,
}

pub enum UpdateMsg {
    FullUpdate(ClientSender),
    Remove(u64),
    SetPixel { x: u32, y: u32, r: u8, g: u8, b: u8 },
    ChatMessage { x: f32, y: f32, text: String },
    Broadcast { text: String },
    SetSize(u32),
}

fn update_thread(rx: mpsc::Receiver<UpdateMsg>, global_lock: Arc<Mutex<GlobalState>>) {
    let canvas_path = env::current_dir().unwrap().join("canvas.place");

    let mut canvas = if let Ok(file) = fs::read(&canvas_path) {
        let c = Canvas::from_file(file).unwrap();
        eprintln!("Loaded {}×{} canvas.place", c.width, c.height);
        c
    } else {
        eprintln!("Failed to read canvas.place, creating blank 500×500");
        Canvas::blank(500, 500)
    };

    let mut last_save = time::Instant::now();
    let mut dirty = false;

    loop {
        let mut messages = Vec::new();
        if let Ok(msg) = rx.recv_timeout(time::Duration::new(5, 0)) {
            messages.push(msg);
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }

        if !messages.is_empty() {
            let mut global = global_lock.lock().unwrap();

            let mut broadcasts = Vec::new();
            let mut full_update = None;
            for message in messages {
                match message {
                    UpdateMsg::FullUpdate(sender) => {
                        let out = Arc::clone(&sender.out);
                        global.clients.insert(sender.id, sender);
                        if full_update.is_none() {
                            let region: messages::RGBARegion = canvas
                                .region(0, 0, canvas.width, canvas.height)
                                .unwrap()
                                .into();
                            full_update = Some(ClientMessage::FullUpdate {
                                w: canvas.width,
                                h: canvas.height,
                                data: region.data,
                            });
                        }
                        match out.send(full_update.clone().unwrap()) {
                            Ok(_) => (),
                            Err(err) => eprintln!("Send error: {:?}", err),
                        }
                    }
                    UpdateMsg::Remove(id) => {
                        global.clients.remove(&id);
                    }
                    UpdateMsg::SetPixel { x, y, r, g, b } => {
                        canvas.set_pixel(x, y, r, g, b);
                        dirty = true;
                    }
                    UpdateMsg::ChatMessage { x, y, text } => {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            broadcasts.push(ClientMessage::ChatMessage {
                                x,
                                y,
                                text,
                                id_hue: None,
                                is_admin: false,
                            });
                        }
                    }
                    UpdateMsg::Broadcast { text } => {
                        broadcasts.push(ClientMessage::Broadcast { text });
                    }
                    UpdateMsg::SetSize(size) => {
                        canvas.set_size(size, size);

                        let region: messages::RGBARegion = canvas
                            .region(0, 0, canvas.width, canvas.height)
                            .unwrap()
                            .into();
                        broadcasts.push(ClientMessage::FullUpdate {
                            w: canvas.width,
                            h: canvas.height,
                            data: region.data,
                        });
                    }
                }
            }

            let mut regions: Vec<messages::RGBARegion> = Vec::new();
            for region in canvas.compile_deltas(Some(MAX_PIXELS_PER_FRAME)) {
                regions.push(region.into());
            }
            let message = ClientMessage::Regions(regions);

            if !global.clients.is_empty() {
                let client = global.clients.iter().next().unwrap().1;
                client.out.broadcast(message).unwrap();

                for broadcast in broadcasts {
                    client.out.broadcast(broadcast).unwrap();
                }
            }
        }

        if last_save.elapsed().as_secs() > 5 && dirty {
            let canvas_data = canvas.to_file();
            let canvas_path = canvas_path.clone();
            thread::spawn(|| {
                match fs::write(canvas_path, canvas_data) {
                    Ok(_) => (),
                    Err(err) => eprintln!("Failed to save! {:?}", err),
                };
            });
            eprintln!("Saving");
            last_save = time::Instant::now();
            dirty = false;
        }

        // wait 16ms for ~60fps
        thread::sleep(time::Duration::new(0, 16_666_667));
    }
}

pub struct GlobalState {
    static_dir: PathBuf,
    clients: HashMap<u64, ClientSender>,
    logins: Logins,
}

impl GlobalState {
    pub fn new() -> GlobalState {
        GlobalState {
            static_dir: env::current_dir()
                .unwrap()
                .join("static")
                .canonicalize()
                .unwrap(),
            clients: HashMap::new(),
            logins: Logins::init(),
        }
    }
}

struct ConnHandler {
    out: Arc<ws::Sender>,
    global: Weak<Mutex<GlobalState>>,
    update_tx: mpsc::Sender<UpdateMsg>,
    id: u64,
    prev_login_attempt: Option<time::Instant>,
    login: Option<String>,
    id_info: String,
}

impl ConnHandler {
    fn new(
        global: Weak<Mutex<GlobalState>>,
        out: ws::Sender,
        update_tx: mpsc::Sender<UpdateMsg>,
        id: u64,
    ) -> ConnHandler {
        let out = Arc::new(out);
        ConnHandler {
            out,
            global,
            update_tx,
            id,
            prev_login_attempt: None,
            login: None,
            id_info: String::new(),
        }
    }

    fn not_found() -> ws::Response {
        ws::Response::new(404, "Not Found", b"Not found".to_vec())
    }

    fn forbidden() -> ws::Response {
        ws::Response::new(403, "Forbidden", b"Forbidden".to_vec())
    }

    fn internal_error() -> ws::Response {
        ws::Response::new(
            500,
            "Internal Server Error",
            b"Internal server error".to_vec(),
        )
    }

    fn send(&self, msg: ClientMessage) {
        match self.out.send(msg) {
            Ok(_) => (),
            Err(err) => eprintln!("Send error: {:?}", err),
        }
    }

    fn send_error(&self, code: &str, message: &str) {
        self.send(ClientMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
        });
    }
}

impl ws::Handler for ConnHandler {
    fn on_request(&mut self, req: &ws::Request) -> ws::Result<(ws::Response)> {
        let mut user_agent = String::from("?");
        for (header, data) in req.headers() {
            if header == "User-Agent" {
                user_agent = String::from_utf8_lossy(data).to_string();
            }
        }
        self.id_info = format!("addr: {:?}, ua: {}", req.client_addr(), user_agent);

        match req.resource() {
            "/canvas" => ws::Response::from_request(req),
            path => {
                let path = if path == "/" { "/index.html" } else { path };

                let global = self.global.upgrade().unwrap();
                let static_dir = &global.lock().unwrap().static_dir;

                let file_path = match static_dir
                    .join(match PathBuf::from(path).strip_prefix("/") {
                        Ok(path) => path,
                        Err(_) => return Ok(ConnHandler::not_found()),
                    })
                    .canonicalize()
                {
                    Ok(path) => path,
                    Err(_) => return Ok(ConnHandler::not_found()),
                };

                if let Err(_) = file_path.strip_prefix(&static_dir) {
                    return Ok(ConnHandler::not_found());
                }

                match fs::read(&file_path) {
                    Ok(file) => {
                        let mut res = ws::Response::new(200, "OK", file);
                        if let Some(ext) = file_path.extension() {
                            if let Some(ext) = ext.to_str() {
                                match ext {
                                    "html" => res.headers_mut().push((
                                        "Content-Type".into(),
                                        b"text/html; charset=utf-8".to_vec(),
                                    )),
                                    "css" => res.headers_mut().push((
                                        "Content-Type".into(),
                                        b"text/css; charset=utf-8".to_vec(),
                                    )),
                                    "js" => res.headers_mut().push((
                                        "Content-Type".into(),
                                        b"application/javascript; charset=utf-8".to_vec(),
                                    )),
                                    _ => (),
                                }
                            }
                        }
                        Ok(res)
                    }
                    Err(err) => match err.kind() {
                        io::ErrorKind::NotFound => Ok(ConnHandler::not_found()),
                        io::ErrorKind::PermissionDenied => Ok(ConnHandler::forbidden()),
                        _ => Ok(ConnHandler::internal_error()),
                    },
                }
            }
        }
    }

    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        self.update_tx
            .send(UpdateMsg::FullUpdate(ClientSender {
                id: self.id,
                id_info: self.id_info.clone(),
                out: Arc::clone(&self.out)
            }))
            .unwrap();
        Ok(())
    }

    fn on_close(&mut self, _: ws::CloseCode, _: &str) {
        self.update_tx.send(UpdateMsg::Remove(self.id)).unwrap();
    }

    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        if let ws::Message::Text(message) = message {
            let client_request: ClientRequest = match serde_json::from_str(&message) {
                Ok(req) => req,
                Err(err) => {
                    self.send_error("message-json", &format!("Invalid message: {}", err));
                    return Ok(());
                }
            };

            match client_request {
                ClientRequest::SetPixel { x, y, r, g, b } => {
                    self.update_tx
                        .send(UpdateMsg::SetPixel { x, y, r, g, b })
                        .unwrap();
                }
                ClientRequest::ChatMessage { x, y, text } => {
                    self.update_tx
                        .send(UpdateMsg::ChatMessage { x, y, text })
                        .unwrap();
                }
                ClientRequest::Auth { login, password } => {
                    if let Some(prev_time) = self.prev_login_attempt {
                        if prev_time.elapsed().as_secs() < 3 {
                            self.send(ClientMessage::Auth(None));
                            return Ok(());
                        }
                    }

                    let global_lock = self.global.upgrade().unwrap();
                    if global_lock
                        .lock()
                        .unwrap()
                        .logins
                        .verify_login(&login, &password)
                    {
                        self.send(ClientMessage::Auth(Some(true)));
                        self.send(ClientMessage::Console(format!("Logged in ({})", login)));
                        self.login = Some(login);
                    } else {
                        self.prev_login_attempt = Some(time::Instant::now());
                        self.send(ClientMessage::Auth(Some(false)));
                    }
                }
                ClientRequest::Console(cmd) => {
                    console::run_command(&self.out, &self.update_tx, &self.global, &cmd);
                }
            }
        } else {
            self.send_error("socket-message-type", "Message type must be text");
        }
        Ok(())
    }
}
