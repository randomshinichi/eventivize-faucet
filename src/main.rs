extern crate futures;
extern crate hyper;

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
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
    listen_addr: SocketAddr,
    chain_id: String,
    cli_binary_path: String,
    cli_config_path: String,
    faucet_addr: String,
    unit: String,
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let config_path = &args[1];
    println!("Searching for {}", config_path);
    let f = File::open(config_path).unwrap();
    let config: Configuration = serde_yaml::from_reader(f).unwrap();
    println!("{:?}", config);

    // Create a closure called router - it's a function that will return another
    // function. This other function will be our HTTP handler.
    let router = || {
        // service_fn_ok() wraps a (HTTP request) handler function inside a
        // service handler
        service_fn_ok(|req| {
            // Here we construct a response. Use pattern matching to match
            // against a tuple of (HTTP_METHOD, HTTP_PATH)
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/ping") => {
                    // json!() is a macro. It turns a JSON object into a Rust
                    // object, then calls .to_string() on it
                    Response::new(Body::from(json!({"message": "pong"}).to_string()))
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
