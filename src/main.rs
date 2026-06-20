#![allow(dead_code)]

mod app;
mod backend_api;
mod backend_daemon;
mod config;
mod domain;
mod error;
mod metadata;
mod player_daemon;
mod repository;
mod service;
mod task;

use crate::app::AppContext;
use crate::backend_api::{
    FrontendEditableSettings, MediaSourceRequest, OpenMediaRequest, export_types,
    frontend_event_from_app, media_source, open_media, save_settings_config, scan, settings_config,
    snapshot,
};
use crate::config::ConfigStore;
use crate::error::{AppResult, io_error};
use crate::task::AppEvent;
use std::io::Read;
use std::sync::mpsc;
use std::thread;

fn main() -> AppResult<()> {
    let config_path = std::env::var("NEXPLAY_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
    let config = ConfigStore::load_or_create(config_path)?;
    let context = AppContext::new(config)?;
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "snapshot".to_string());

    match command.as_str() {
        "snapshot" => print_json(&snapshot(&context)?)?,
        "scan" => {
            start_event_forwarder(&context);
            print_json(&scan(&context)?)?;
        }
        "settings" => print_json(&settings_config(&context)?)?,
        "save-settings" => {
            let mut raw = String::new();
            std::io::stdin()
                .read_to_string(&mut raw)
                .map_err(|err| io_error("<stdin>", err))?;
            let input: FrontendEditableSettings = serde_json::from_str(&raw)?;
            print_json(&save_settings_config(&context, input)?)?;
        }
        "open-media" => {
            let mut raw = String::new();
            std::io::stdin()
                .read_to_string(&mut raw)
                .map_err(|err| io_error("<stdin>", err))?;
            let input: OpenMediaRequest = serde_json::from_str(&raw)?;
            print_json(&open_media(&context, input)?)?;
        }
        "media-source" => {
            let mut raw = String::new();
            std::io::stdin()
                .read_to_string(&mut raw)
                .map_err(|err| io_error("<stdin>", err))?;
            let input: MediaSourceRequest = serde_json::from_str(&raw)?;
            print_json(&media_source(&context, input)?)?;
        }
        "backend-daemon" => backend_daemon::run_backend_daemon(context)?,
        "export-types" => {
            let output_path = std::env::args()
                .nth(2)
                .unwrap_or_else(|| "frontend/src/generated/backend.ts".to_string());
            export_types(output_path)?;
        }
        "player-daemon" => player_daemon::run_player_daemon()?,
        "help" | "--help" | "-h" => {
            println!("NexPlay backend commands:");
            println!("  snapshot  print the current library snapshot as JSON");
            println!("  scan      scan configured media libraries and print JSON");
            println!("  settings  print editable settings as JSON");
            println!(
                "  save-settings  read editable settings JSON from stdin and write config.toml"
            );
            println!("  open-media  read media id JSON from stdin and open with default player");
            println!("  media-source  read media id JSON from stdin and print playback source");
            println!("  backend-daemon  run a persistent JSON-RPC backend over stdio");
            println!("  export-types [path]  generate frontend TypeScript DTOs");
            println!("  player-daemon  run a persistent libmpv JSON-lines control process");
        }
        other => {
            return Err(crate::error::AppError::Config(format!(
                "unknown backend command: {other}"
            )));
        }
    }

    Ok(())
}

fn print_json<T: serde::Serialize>(value: &T) -> AppResult<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn start_event_forwarder(context: &AppContext) {
    let Some(receiver) = context
        .event_receiver
        .lock()
        .expect("event receiver mutex poisoned")
        .take()
    else {
        return;
    };

    thread::spawn(move || forward_events(receiver));
}

fn forward_events(receiver: mpsc::Receiver<AppEvent>) {
    for event in receiver {
        let value = serde_json::to_value(frontend_event_from_app(event)).unwrap_or_else(
            |error| serde_json::json!({ "type": "metadataFailed", "message": error.to_string() }),
        );

        eprintln!("{value}");
    }
}
