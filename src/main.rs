use reqwest::Proxy;
use std::env;
use std::fs;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use colored::*;
use serde_json::Value;

async fn check_proxy(proxy: String, counter: Arc<Mutex<usize>>, total_proxies: usize) {
    let proxy_parts: Vec<&str> = proxy.split(":").collect();
    if proxy_parts.len() != 2 {
        let mut counter = counter.lock().unwrap();
        *counter += 1;
        return;
    }

    let ip_port = format!("{}:{}", proxy_parts[0], proxy_parts[1]);

    let config_content = match fs::read_to_string("config.json") {
        Ok(content) => content,
        Err(_) => {
            println!("[{}] Error reading config file.", "LOG*".purple());
            return;
        }
    };

    let json_data: Value = match serde_json::from_str(&config_content) {
        Ok(data) => data,
        Err(err) => {
            println!("[{}] Error parsing config file. {}", "LOG*".purple(), err);
            return;
        }
    };

    let link = match json_data["main"]["settings"]["debug"]["Link"].as_str() {
        Some(v) => v,
        None => {
            println!("[{}] Version not specified in config file.", "LOG*".purple());
            return;
        }
    };
    let start_time = Instant::now();

    if let Ok(stream) = TcpStream::connect(&ip_port) {
        let proxy = Proxy::http(&proxy).unwrap();
        let client = reqwest::Client::builder().proxy(proxy.clone()).build().unwrap();

        match client.get(link).send().await {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis();
                println!("\r[{}] Proxy {} is {}. Connection time: {} ms", "REQ*".purple(), ip_port, "working".bright_green(), duration);
            }
            Err(err) => {
                if err.to_string().contains("authentication") {
                    println!("\r[{}] Proxy {} {}. (requires authentication)", "REQ*".purple(), ip_port, "not working".bright_red());
                } else {
                    println!("\r[{}] Proxy {} is {}.", "REQ*".purple(), ip_port, "not working".bright_red());
                }
            }
        }
    } else {
        println!("\r[{}] Proxy {} is {}. Connection failed.", "REQ*".purple(), ip_port, "not working".bright_red());
    }

    {
        let mut counter = counter.lock().unwrap();
        *counter += 1;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("{} Usage: {} {} <value_num> [{}] {} <value_file.txt>", "[LOG*]".purple(), "./proxychecker".purple(), "-threads".purple(), "-fast".purple(), "-proxy".purple());
        return;
    }

    let config_content = match fs::read_to_string("config.json") {
        Ok(content) => content,
        Err(_) => {
            println!("[{}] Error reading config file.", "LOG*".purple());
            return;
        }
    };

    let json_data: Value = match serde_json::from_str(&config_content) {
        Ok(data) => data,
        Err(err) => {
            println!("[{}] Error parsing config file. {}", "LOG*".purple(), err);
            return;
        }
    };

    let version = match json_data["main"]["settings"]["debug"]["Version"].as_str() {
        Some(v) => v,
        None => {
            println!("[{}] Version not specified in config file.", "LOG*".purple());
            return;
        }
    };

    let owner = match json_data["main"]["settings"]["debug"]["Owner"].as_str() {
        Some(o) => o,
        None => {
            println!("[{}] Owner not specified in config file.", "LOG*".purple());
            return;
        }
    };

    println!("[{}] Version: {}", "LOG*".purple(), version);
    println!("[{}] Owner: {}", "LOG*".purple(), owner);
    println!("[{}] FAST MODE ENABLED\n\n", "LOG*".purple());

    let mut num_threads: usize = 1;
    let mut fast_mode = false;
    let mut proxy_file_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-threads" => {
                if i + 1 < args.len() {
                    num_threads = args[i + 1].parse().unwrap_or(1);
                    i += 2;
                } else {
                    println!("[{}] Invalid argument: -threads requires a value.", "LOG*".purple());
                    return;
                }
            }
            "-fast" => {
                println!("[{}] FAST MODE ENABLED", "LOG*".purple());
                fast_mode = true;
                i += 1;
            }
            "-proxy" => {
                if i + 1 < args.len() {
                    proxy_file_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    println!("[{}] Invalid argument: -proxy requires a value.", "LOG*".purple());
                    return;
                }
            }
            _ => {
                println!("[{}] Invalid argument: {}", "LOG*".purple(), args[i]);
                return;
            }
        }
    }

    if let Some(path) = proxy_file_path {
        if let Ok(proxy_content) = fs::read_to_string(&path) {
            let proxies: Vec<String> = proxy_content.lines().map(|s| s.to_owned()).collect();
            let total_proxies = proxies.len();
            let counter = Arc::new(Mutex::new(0));
            let mut handles = vec![];
            for proxy in proxies {
                let counter = Arc::clone(&counter);
                let handle = thread::spawn(move || {
                    tokio::runtime::Runtime::new().unwrap().block_on(check_proxy(proxy, counter, total_proxies));
                });
                handles.push(handle);
            }
            for handle in handles {
                handle.join().unwrap();
            }

            let counter = counter.lock().unwrap();
            println!("\n\n[{}] {} proxies: {}", "INFO*".purple(), "Working".bright_green(), *counter);
            println!("[{}] {} proxies: {}", "INFO*".purple(), "Not working".bright_red(), total_proxies - *counter);
        } else {
            println!("[{}] Error reading proxy file: {}.", "LOG*".purple(), path);
        }
    } else {
        println!("[{}] Proxy file not specified.", "LOG*".purple());
    }
}
