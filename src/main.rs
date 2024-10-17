use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{fs, time::Duration};
use reqwest::blocking::Client;
use std::sync::mpsc::channel;
use std::path::Path;
use config::{Config};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Settings {
    screenshots_folder: String,
    webhook_url: String,
}

fn send_discord_notification(message: &str, webhook_url: &str) {
    let client = Client::new();
    let payload = serde_json::json!({
        "content": message
    });

    let response = client
        .post(webhook_url)
        .json(&payload)
        .send();

    match response {
        Ok(_) => println!("Notification sent!"),
        Err(err) => println!("Failed to send notification: {:?}", err),
    }
}

fn extract_time(input: &str) -> Option<String> {
    println!("{}", input);
    let time_keyword = "[\"time\"] = \"";
    if let Some(start) = input.find(time_keyword) {
        println!("{}", start);
        let start = start + time_keyword.len();
        if let Some(end) = input[start..].find("\"") {
            println!("{}", end);
            let time_str = &input[start..start + end];
            return Some(time_str.to_string());
        }
    }
    None
}

fn read_saved_variables(file_path: &Path) -> Option<String> {
    println!("{:?}", file_path.to_str());
    match  fs::read_to_string(file_path) {
        Ok(content) => {
            extract_time(&*content)
        }
        Err(e) => {
            println!("{}", e);
            None
        }
    }
}

fn main() {
    let mut settings = Config::builder().add_source(config::File::with_name("Settings"))
        .build()
        .unwrap();
    let config: Settings = settings.try_deserialize().unwrap();
    println!("{:#?}", config); // to check the values are loaded correctly
    let screenshots_folder = Path::new(&config.screenshots_folder);
    let (tx, rx) = channel();
    let durtation =   Duration::from_secs(4);
    let notify_config = notify::Config::default().with_poll_interval(durtation);
    let mut watcher: RecommendedWatcher = Watcher::new(tx, notify_config).unwrap();
    watcher.watch(screenshots_folder, RecursiveMode::NonRecursive).unwrap();

    println!("Monitoring SavedVariables for queue pops...");
    let mut last_timestamp = "".to_string();

    loop {
        match rx.recv() {
            Ok(_) => {
                if let Ok(entries) = fs::read_dir(screenshots_folder) {
                    println!("Detected change in Screenshots folder.");

                    if let Some(newest_entry) = entries
                        .filter_map(Result::ok)
                        .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap()) {

                        let screenshot_path = newest_entry.path();
                        if let Some(screenshot_name) = screenshot_path.file_name() {
                            let message = format!("Queue pop detected! Screenshot saved: {:?}", screenshot_name);
                            send_discord_notification(&message, &config.webhook_url);
                        }
                    }
                }
            }
            Err(e) => println!("Watch error: {:?}", e),
        }
    }
}
