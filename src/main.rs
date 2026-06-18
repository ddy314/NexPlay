#![allow(dead_code)]

mod app;
mod backend_api;
mod config;
mod domain;
mod error;
mod metadata;
mod repository;
mod service;
mod task;

use crate::app::AppContext;
use crate::backend_api::{
    FrontendEditableSettings, OpenMediaRequest, open_media, save_settings_config, scan,
    settings_config, snapshot,
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
        "help" | "--help" | "-h" => {
            println!("NexPlay backend commands:");
            println!("  snapshot  print the current library snapshot as JSON");
            println!("  scan      scan configured media libraries and print JSON");
            println!("  settings  print editable settings as JSON");
            println!(
                "  save-settings  read editable settings JSON from stdin and write config.toml"
            );
            println!("  open-media  read media id JSON from stdin and open with default player");
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
        let value = match event {
            AppEvent::Log(message) => serde_json::json!({
                "type": "log",
                "message": message,
            }),
            AppEvent::ScanStarted => serde_json::json!({
                "type": "scanStarted",
                "message": "扫描已开始",
            }),
            AppEvent::ScanProgress { scanned, indexed } => serde_json::json!({
                "type": "scanProgress",
                "scanned": scanned,
                "indexed": indexed,
                "message": format!("已扫描 {scanned} 个文件"),
            }),
            AppEvent::ScanFinished { summary, .. } => serde_json::json!({
                "type": "scanFinished",
                "summary": summary,
                "message": format!("文件扫描完成：{} 个文件", summary.scanned_files),
            }),
            AppEvent::ScanFailed(error) => serde_json::json!({
                "type": "scanFailed",
                "message": error,
            }),
            AppEvent::DanmakuMatched(match_result) => serde_json::json!({
                "type": "danmakuMatched",
                "message": match_result.title,
            }),
            AppEvent::MetadataMatchStarted { media_id } => serde_json::json!({
                "type": "metadataStarted",
                "mediaId": media_id,
            }),
            AppEvent::MetadataMatchProgress { processed, total } => serde_json::json!({
                "type": "metadataProgress",
                "processed": processed,
                "total": total,
                "message": format!("元数据整理 {processed}/{total}"),
            }),
            AppEvent::MetadataMatchFinished {
                media_id,
                subject_id,
                title,
            } => serde_json::json!({
                "type": "metadataFinished",
                "mediaId": media_id,
                "subjectId": subject_id,
                "message": title.unwrap_or_else(|| format!("media #{media_id}")),
            }),
            AppEvent::SubjectUpdated { subject_id } => serde_json::json!({
                "type": "subjectUpdated",
                "subjectId": subject_id,
            }),
            AppEvent::ImageCached {
                subject_id,
                image_kind,
            } => serde_json::json!({
                "type": "imageCached",
                "subjectId": subject_id,
                "imageKind": image_kind,
            }),
            AppEvent::MetadataFailed { target_id, error } => serde_json::json!({
                "type": "metadataFailed",
                "targetId": target_id,
                "message": error,
            }),
            AppEvent::MetadataStatus(message) => serde_json::json!({
                "type": "metadataStatus",
                "message": message,
            }),
        };

        eprintln!("{value}");
    }
}
