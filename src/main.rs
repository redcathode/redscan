use std::{env, fs, time::Duration};
use craftping::{Response, tokio::ping};
use tokio::net::TcpStream;

async fn attempt_server_ping(hostname: &str, port: u16) -> Result<Response, &str> {
    match TcpStream::connect((hostname, port)).await {
        Ok(mut stream) => {
            match ping(&mut stream, hostname, port).await {
                Ok(pong) => Ok(pong),
                Err(_) => Err("couldn't ping")
            }
        }
        Err(_) => Err("couldn't connect")
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = dbg!(args.get(1).expect("pass text file of IPs as first argument"));
    let lines: Vec<String> = fs::read_to_string(file_path)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();


    for line in lines {
        tokio::spawn(async move {
            let response = match tokio::time::timeout(
                Duration::from_millis(250),
                attempt_server_ping(&line, 25565)
            ).await {
                Ok(ok) => {
                    if let Ok(pong) = ok {
                        Ok(pong.description)
                    } else {
                        Err("conn")
                    }
                },
                Err(_) => {
                    Err("timer")
                }
            };
            if let Ok(server) = response {
                println!("{}: {:?}", line, server);
            }
            
        });
    }
}
