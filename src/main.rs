#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
use std::env;
use std::fmt;
use std::error;
use std::fs::File;
use std::net::SocketAddr;
use std::process::{Command};
use rocket::request::Form;
use rocket::response::status;
use rocket::response::Responder;
lazy_static! {
    pub static ref CONFIG: Configuration = get_config();
}

#[derive(Debug)]
struct AuthError {
    details: String,
}
impl AuthError {
    fn new(msg: &str) -> AuthError {
        AuthError {
            details: msg.to_string(),
        }
    }
}
impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl error::Error for AuthError {
    fn description(&self) -> &str {
        &self.details
    }
}
impl<'r> Responder<'r> for AuthError {
    fn respond_to(self, _: &rocket::Request) ->rocket::response::Result<'r> {
        Err(rocket::http::Status::new(401, "BAD TOKEN"))
    }
}
#[derive(Debug)]
struct CommandError {
    details: String,
}
impl CommandError {
    fn new(msg: String) -> CommandError {
        CommandError {
            details: msg,
        }
    }
}
impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl error::Error for CommandError {
    fn description(&self) -> &str {
        &self.details
    }
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

fn run_command(c: String) -> Result<String, Box<dyn error::Error>>{
    println!("{}", c);
    let c_vec: Vec<&str> = c.split(" ").collect();
    let binary = c_vec[0];
    let output = Command::new(binary)
        .args(&c_vec[1..])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    if !output.status.success() {
        return Err(Box::new(CommandError::new(stderr)));
    }
    return Ok(stdout)
}

fn send_tx(config: &Configuration, dest_addr: String, amount: String) -> Result<String, Box<dyn error::Error>> {
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

fn run_status(config: &Configuration) -> Result<String, Box<dyn error::Error>> {
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
fn http_status() -> Result<String, Box<dyn error::Error>>{
    let o = run_status(&CONFIG);
    return o;
}
#[derive(FromForm, Debug)]
struct SendAuth {
    token: String,
}

#[post("/send/<to_address>/<amount>", data="<auth>")]
fn http_send(to_address: String, amount: String, auth: Form<SendAuth>) -> Result<String, Box<dyn error::Error>> {
    println!("{:?}", auth);
    if auth.token != CONFIG.secret {
        return Err(Box::new(AuthError::new("Your token was wrong")));
    }
    let output = send_tx(&CONFIG, to_address, amount);
    println!("Output {:?}", output);
    return output
}
fn main() {
    rocket::ignite()
        .mount("/", routes![index, http_status, http_send])
        .launch();
}
