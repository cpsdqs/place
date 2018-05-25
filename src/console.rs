use messages::ClientMessage;
use std::collections::HashMap;
use std::mem;
use std::sync::mpsc::Sender;
use std::sync::{Weak, Mutex};
use ws;
use {UpdateMsg, GlobalState};

/// Splits the string into parts, respecting quoted text.
fn split_command(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut quotes = None;
    let mut prev_was_backslash = false;
    let mut part = String::new();

    fn push_part(parts: &mut Vec<String>, part: &mut String) {
        if !part.is_empty() {
            parts.push(mem::replace(part, String::new()));
        }
    }

    for c in cmd.chars() {
        if quotes.is_none() && c.is_whitespace() {
            push_part(&mut parts, &mut part);
        } else if Some(c) == quotes && !prev_was_backslash {
            quotes = None;
            push_part(&mut parts, &mut part);
        } else if !prev_was_backslash && quotes == None && (c == '\'' || c == '"') {
            push_part(&mut parts, &mut part);
            quotes = Some(c);
        } else if c != '\\' || prev_was_backslash {
            part.push(c);
        }

        prev_was_backslash = c == '\\';
    }
    push_part(&mut parts, &mut part);

    parts
}

/// A call.
struct CmdCall {
    command: String,

    /// Parameters like `--a b`
    params: HashMap<String, String>,

    /// Arguments.
    args: Vec<String>,
}

/// Parses command parts (see above).
fn parse_parts(mut parts: Vec<String>) -> Option<CmdCall> {
    let command = match parts.get(0) {
        Some(c) => c.to_string(),
        None => return None,
    };
    parts.remove(0);
    let mut params = HashMap::new();
    let mut args = Vec::new();
    let mut param_name = None;
    for part in parts {
        if part.starts_with("--") {
            param_name = Some(part[2..].to_string());
        } else if let Some(param) = param_name {
            params.insert(param, part);
            param_name = None;
        } else {
            args.push(part);
        }
    }
    Some(CmdCall {
        command,
        params,
        args,
    })
}

/// Runs a command in the given “context” (out, update_tx, global_weak).
pub fn run_command(
    out: &ws::Sender,
    update_tx: &Sender<UpdateMsg>,
    global_weak: &Weak<Mutex<GlobalState>>,
    command: &str,
) {
    let call = match parse_parts(split_command(command)) {
        Some(call) => call,
        None => return,
    };

    let send_line = |line: &str| match out.send(ClientMessage::Console(line.to_string())) {
        Ok(_) => (),
        Err(err) => eprintln!("Send error: {:?}", err),
    };

    match &*call.command {
        "help" => {
            send_line("Commands: set-size, broadcast, list-clients");
        }
        "set-size" => {
            if call.args.len() < 1 {
                send_line("set-size <size>");
                return;
            }
            let size: u32 = match call.args[0].parse() {
                Ok(s) => s,
                Err(_) => return send_line("set-size <size: integer>"),
            };
            update_tx.send(UpdateMsg::SetSize(size)).unwrap();
        }
        "broadcast" => {
            if call.args.len() < 1 {
                send_line("broadcast <message>");
                return;
            }
            update_tx
                .send(UpdateMsg::Broadcast {
                    text: call.args[0].clone(),
                })
                .unwrap();
        }
        "list-clients" => {
            let global_lock = global_weak.upgrade().unwrap();
            let global = global_lock.lock().unwrap();

            for (_, client) in &global.clients {
                send_line(&client.id_info);
            }
        }
        _ => send_line("Unknown command, `help` for help"),
    }
}
