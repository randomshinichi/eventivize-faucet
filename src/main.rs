#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::process::Command;
use rocket::request::Form;

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
    println!("{}", c);
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

fn run_status(config: &Configuration) -> Result<String, String> {
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

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/status")]
fn http_status() -> Result<String, String> {
    let output = run_status(&CONFIG);
    return output
}
#[derive(FromForm, Debug)]
struct SendAuth {
    token: String,
}

#[post("/send/<to_address>/<amount>", data="<auth>")]
fn http_send(to_address: String, amount: String, auth: Form<SendAuth>) -> Result<String, String> {
    println!("{:?}", auth);
    if auth.token != CONFIG.secret {
        return Err(String::from("Your token was wrong"));
    }
    let output = send_tx(&CONFIG, to_address, amount);
    return output
}
fn main() {
    rocket::ignite()
        .mount("/", routes![index, http_status, http_send])
        .launch();
}
