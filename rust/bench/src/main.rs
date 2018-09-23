#[macro_use]
extern crate serde_json;

extern crate clap;
extern crate serde;
extern crate xi_core_lib;

use clap::{Arg, App, SubCommand};
use serde::ser::{Serialize, Serializer};
use std::io::prelude::*;
use std::io::BufReader;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use xi_core_lib::rpc;

type RpcIndex = u64;
struct XiCore {
    stdin: ChildStdin,
    rpc_rx: mpsc::Receiver<serde_json::Value>,
    rpc_index: u64,
}

impl XiCore {
    pub fn new(executable: &str) -> XiCore {
        let process = Command::new(executable)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .env("RUST_BACKTRACE", "1")
            .spawn()
            .unwrap_or_else(|e| panic!("failed to execute core: {}", e));
        // Communicate via a channel with a thread that receives results and
        // notifications from xi-core.
        let (rpc_tx, rpc_rx) = mpsc::channel();
        let stdout = process.stdout.unwrap();
        thread::spawn(move || {
            println!("reading from stdout");
            for line in BufReader::new(stdout).lines() {
                println!("[bench] response {:?}", line);
                let line = line.unwrap();
                let value = serde_json::from_str::<serde_json::Value>(&line).unwrap();
                let obj = value.as_object().unwrap();
                if obj.get("id").is_some() {
                    // proper response, we have to distinguish between Result and Error
                    rpc_tx.send(value.clone()).unwrap();
                } else {
                    eprintln!("[bench] notification {:?}", value);
                }
            }
        });
        // A thread is handling stderr and just puts everything onto our stderr.
        // TODO: This implementation does not synchronize writes to stderr.
        let stderr = process.stderr.unwrap();
        thread::spawn(move || {
            let buf_reader = BufReader::new(stderr);
            for line in buf_reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[core] {}", line);
                }
            }
        });

        XiCore {
            stdin: process.stdin.unwrap(),
            rpc_rx: rpc_rx,
            rpc_index: 0,
        }
    }

    /// Serialize a xi_core_lib::rpc call and send it to xi-core.
    pub fn send_request<T: Serialize>(&mut self, v: T) -> Result<RpcIndex, &'static str> {
        self.rpc_index += 1;
        let req = RpcRequest(v, self.rpc_index);
        let mut msg = serde_json::to_string(&req).unwrap();
        println!("[bench] send: {}", msg);
        msg.push('\n');
        self.stdin
            .write_all(msg.as_bytes())
            .or_else(|_e| Err("can't write"))?;

        Ok(self.rpc_index)
    }

    /// Serialize a xi_core_lib::rpc call and send it to xi-core.
    pub fn send<T: Serialize>(&mut self, v: T) -> Result<RpcIndex, &'static str> {
        let mut msg = serde_json::to_string(&v).unwrap();
        println!("[bench] send: {}", msg);
        msg.push('\n');
        self.stdin
            .write_all(msg.as_bytes())
            .or_else(|_e| Err("can't write"))?;

        Ok(self.rpc_index)
    }
    pub fn sync_request<T>(&mut self, v: T) -> Result<serde_json::Value, &'static str>
    where
        T: Serialize,
    {
        self.send_request(v).and_then(|_e| self.recv())
    }

    pub fn sync<T>(&mut self, v: T) -> Result<serde_json::Value, &'static str>
    where
        T: Serialize,
    {
        self.send(v).and_then(|_e| self.recv())
    }

    pub fn recv(&self) -> Result<serde_json::Value, &'static str> {
        self.rpc_rx.recv().or_else(|_e| Err("failure"))
    }
}

/// An RPC requests consist of an serialize xi_core_lib::rpc enum and an
/// additional id.
struct RpcRequest<T: Serialize>(T, RpcIndex);
impl<T: Serialize> Serialize for RpcRequest<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut value = serde_json::to_value(&self.0).unwrap();
        value["id"] = json!(self.1);
        value.serialize(serializer)
    }
}

fn main() {

    let matches = App::new("xi-bench").arg(Arg::with_name("executable")
        .long("xi_executable")
        .value_name("EXECUTABLE")
        .takes_value(true)).get_matches();
    let xi_executable = matches.value_of("executable").unwrap_or("xi-core");
    let mut xi = XiCore::new(xi_executable);
    xi.send(rpc::CoreNotification::ClientStarted {
        config_dir: None,
        client_extras_dir: None,
    }).unwrap();

    let result = xi
        .sync_request(rpc::CoreRequest::NewView { file_path: None })
        .expect("Error setting up NewView");

    let viewid = result["result"].as_str().unwrap().into();
    xi.send(rpc::CoreNotification::Edit(rpc::EditCommand {
        view_id: viewid,
        cmd: rpc::EditNotification::Insert { chars: "f".into() },
    })).unwrap();

    xi.send(rpc::CoreNotification::Save {
        view_id: viewid,
        file_path: "test.xi".into(),
    }).unwrap();
    xi.recv();
}
