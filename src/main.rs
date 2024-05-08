use std::{env, fs, time::Duration, fmt::Write};
use craftping::{Response, tokio::ping};
use tokio::net::TcpStream;
use tokio_task_pool::Pool;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

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
        .with_spawn_timeout(Duration::from_millis(50))
        .with_run_timeout(Duration::from_millis(250));
    
    let lines: Vec<String> = fs::read_to_string(file_path)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();

    let num_hosts = lines.len();
    let mut host_num: usize = 0;

    let pb: ProgressBar = ProgressBar::new(num_hosts as u64);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} hosts ({percent_precise}% - eta {eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));


    for line in lines {
        host_num += 1;
        pool.spawn(async move {
            let response = match attempt_server_ping(&line, 25565).await {
                Ok(pong) => Ok(format!("desc: {:?}, secure_chat: {:?}, online: {}, max: {}, version: {}, protocol: {}", pong.description, pong.enforces_secure_chat, pong.online_players, pong.max_players, pong.version, pong.protocol)),
                Err(_) => Err("pong")
            };
            if let Ok(server) = response {
                let tbp = format!("{}: {}", line, server);
                println!("{}", tbp);
                eprintln!("\r{:.3}% ({}/{} hosts) - {}", (host_num as f32 / num_hosts as f32) * 100.0, host_num, num_hosts, tbp);
            }
            
        }).await.unwrap();
        pb.set_position(host_num as u64);
    }
}
