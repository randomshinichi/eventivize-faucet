extern crate futures;
extern crate hyper;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;

use futures::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Method, Response, Server, StatusCode};
use std::env;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;

lazy_static! {
    pub static ref CONFIG: Configuration = { get_config() };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    listen_addr: SocketAddr,
    chain_id: String,
    cli_binary_path: String,
    cli_config_path: String,
    faucet_addr: String,
    unit: String,
    node_addr: String,
}

fn run_command(c: String) -> (bool, String, String) {
    let c_vec: Vec<&str> = c.split(" ").collect();
    let binary = c_vec[0];
    let output = Command::new(binary)
        .args(&c_vec[1..])
        .output()
        .expect("Could not execute launchpayloadcli command");

    let stdout = String::from_utf8(output.stdout).expect("Found invalid UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
    println!("{}\n{}", stdout, stderr);
    return (output.status.success(), stdout, stderr);
}

fn send_tx(config: &Configuration) -> (bool, String, String) {
    let cli_options = format!(
        "--home {} --keyring-backend test --chain-id {}",
        config.cli_config_path, config.chain_id
    );

    let amount = "500drop";
    let dest_addr = "cosmosBLABLA";
    let cli_send = format!(
        "{} tx send {} {} {} {} --yes",
        config.cli_binary_path, config.faucet_addr, dest_addr, amount, cli_options
    );
    return run_command(cli_send);
}

fn status(config: &Configuration) -> (bool, String, String) {
    let cli_status = format!(
        "{} status --node tcp://{}",
        config.cli_binary_path, config.node_addr
    );
    return run_command(cli_status);
}

fn get_config() -> Configuration {
    let args: Vec<String> = env::args().collect();
    let config_path = &args[1];
    let f = File::open(config_path).unwrap();
    let config: Configuration = serde_yaml::from_reader(f).unwrap();
    return config;
}

fn main() {
    // Create a closure called router - it's a function that will return another
    // function. This other function will be our HTTP handler.
    let router = || {
        // service_fn_ok() wraps a (HTTP request) handler function inside a
        // service handler
        service_fn_ok(|req| {
            // Here we construct a response. Use pattern matching to match
            // against a tuple of (HTTP_METHOD, HTTP_PATH)
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/status") => {
                    // json!() is a macro. It turns a JSON object into a Rust
                    // object, then calls .to_string() on it
                    let (success, stdout, stderr) = status(&CONFIG);
                    Response::new(Body::from(
                        json!({"success": success, "stdout": stdout, "stderr": stderr}).to_string(),
                    ))
                }
                (_, _) => {
                    let mut res = Response::new(Body::from("not found"));

                    // tell me where you keep your status code, so I can change
                    // it to my new 404 code
                    *res.status_mut() = StatusCode::NOT_FOUND;
                    res
                }
            }
        })
    };

    let addr = "127.0.0.1:8080".parse().unwrap(); // Rust knows this will be a SocketAddr because this variable is later used in a function that takes a SocketAddr
    let server = Server::bind(&addr).serve(router); // we are letting bind() use the &addr reference, but not own it. This also means addr is immutable to bind()
    hyper::rt::run(server.map_err(|e| {
        eprintln!("server error: {}", e);
    }))
}
