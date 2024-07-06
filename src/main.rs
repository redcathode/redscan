use std::{env, fs, time::Duration, fmt::Write};
use craftping::{Response, tokio::ping};
use tokio::net::TcpStream;
use tokio_task_pool::Pool;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use sqlx::SqlitePool;

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
    let max_concurrent_hosts: usize = dbg!(args.get(2).unwrap_or(&"200".to_string()).parse().unwrap());
    let pool = Pool::bounded(max_concurrent_hosts)
        .with_run_timeout(Duration::from_millis(250));

    let lines: Vec<String> = fs::read_to_string(file_path)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();

    let num_hosts = lines.len();
    let mut host_num: usize = 0;
    let sqlitepool = SqlitePool::connect("sqlite://./servers.db").await.unwrap();

    let pb: ProgressBar = ProgressBar::new(num_hosts as u64);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} hosts ({percent_precise}% - eta {eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    let mut handles = vec![];

    for line in lines {
    host_num += 1;
    let sqlitepool_clone = sqlitepool.clone(); // Clone the pool for use in the async block.
    handles.push(tokio::spawn(async move { // Use tokio::spawn for asynchronous execution.
        let response = match attempt_server_ping(&line, 25565).await {
            Ok(pong) => Ok((
                line, // IP address or server identifier
                25565 as i32,
                pong.description.text, // Description
                pong.enforces_secure_chat, // Secure chat
                pong.online_players, // Online players
                pong.max_players, // Max players
                pong.version, // Version
                pong.protocol // Protocol
            )),
            Err(_) => Err("Failed to ping server")
        };

        if let Ok((ip_address, port, description, secure_chat, online_players, max_players, version, protocol)) = response {
            let insert_query = sqlx::query("INSERT INTO servers (ip_address, port, description, secure_chat, online_players, max_players, version, protocol) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&ip_address)
                .bind(port)
                .bind(description)
                .bind(secure_chat)
                .bind(online_players as i32)
                .bind(max_players as i32)
                .bind(version)
                .bind(protocol);
            println!("{}:{}: {} players online", ip_address, port, online_players);
            match insert_query.execute(&sqlitepool_clone).await {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to insert server details: {}", e),
            }
        }
    }));
    pb.set_position(host_num as u64);
}
futures::future::join_all(handles).await; // Ensure to handle errors appropriately in your actual code.
}
