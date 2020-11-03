#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::process::Command;
use warp::Filter;

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
    secret: String, // make sure random people can't call the faucet
}

fn run_command(c: String) -> Result<String, String> {
    let c_vec: Vec<&str> = c.split(" ").collect();
    let binary = c_vec[0];
    let output = Command::new(binary)
        .args(&c_vec[1..])
        .output()
        .expect("Could not execute launchpayloadcli command");

    let stdout = String::from_utf8(output.stdout).expect("Found invalid UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
    println!("{}\n{}", stdout, stderr);
    match output.status.success() {
        true => Ok(stdout),
        false => Err(stderr),
    }
}

fn send_tx(config: &Configuration, dest_addr: String, amount: String) -> Result<String, String> {
    let cli_options = format!(
        "--home {} --keyring-backend test --chain-id {} --node tcp://{} -o json",
        config.cli_config_path, config.chain_id, config.node_addr
    );

    let cli_send = format!(
        "{} tx send {} {} {} {} --yes",
        config.cli_binary_path, config.faucet_addr, dest_addr, amount, cli_options
    );
    return run_command(cli_send);
}

fn status(config: &Configuration) -> Result<String, String> {
    let cli_status = format!(
        "{} status --node tcp://{} -o json",
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

#[tokio::main]
async fn main() {
    // GET /status
    let status = warp::path!("status").map(|| {
        let ans = status(&CONFIG).unwrap();
        let ans_j: serde_json::Value = serde_json::from_str(&ans).unwrap();
        warp::reply::json(&ans_j)
    });

    // POST /send/cosmosaddr/amount
    let send = warp::post().and(warp::path!("send" / String / String)).map(
        |dest_addr: String, amount: String| {
            println!("Oh man a POST happened! with param {}", dest_addr);
            let ans = send_tx(&CONFIG, dest_addr, amount).unwrap();
            let ans_j: serde_json::Value = serde_json::from_str(&ans).unwrap();
            warp::reply::json(&ans_j)
        },
    );

    let routes = warp::get().and(status).or(warp::post().and(send));
    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
