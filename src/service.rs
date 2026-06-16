use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{ConfigStore, DandanplayConfig};
use crate::domain::{DanmakuMatch, MediaItem, WatchProgress};
use crate::error::{AppError, AppResult};
use crate::repository::Repository;
use crate::task::{self, AppEvent};

#[derive(Clone)]
pub struct MediaService {
    config: Arc<ConfigStore>,
    repository: Repository,
    events: mpsc::Sender<AppEvent>,
}

impl MediaService {
    pub fn new(
        config: Arc<ConfigStore>,
        repository: Repository,
        events: mpsc::Sender<AppEvent>,
    ) -> Self {
        Self {
            config,
            repository,
            events,
        }
    }

    pub fn add_library_path(&self, path: PathBuf) -> AppResult<Vec<PathBuf>> {
        let paths = self.config.add_media_library(path)?;
        self.send_log(format!("media library paths: {}", paths.len()));
        Ok(paths)
    }

    pub fn list_media(&self) -> AppResult<Vec<MediaItem>> {
        self.repository.list_media(false)
    }

    pub fn start_scan(&self) {
        let roots = self.config.snapshot().media_libraries;
        if roots.is_empty() {
            self.send_log("no media library paths configured".to_string());
            return;
        }

        task::spawn_media_scan(self.repository.clone(), roots, self.events.clone());
    }

    fn send_log(&self, message: String) {
        let _ = self.events.send(AppEvent::Log(message));
    }
}

#[derive(Clone)]
pub struct WatchHistoryService {
    repository: Repository,
    events: mpsc::Sender<AppEvent>,
}

impl WatchHistoryService {
    pub fn new(repository: Repository, events: mpsc::Sender<AppEvent>) -> Self {
        Self { repository, events }
    }

    pub fn load(&self, media_id: i64) -> AppResult<Option<WatchProgress>> {
        self.repository.get_progress(media_id)
    }

    pub fn save_test_progress(&self, media_id: i64) -> AppResult<WatchProgress> {
        let now = task::unix_timestamp_ms();
        let position_ms = 15 * 60 * 1000;
        let duration_ms = 24 * 60 * 1000;
        self.repository
            .save_progress(media_id, position_ms, duration_ms, now)?;

        let progress = WatchProgress {
            media_id,
            position_ms,
            duration_ms,
            updated_at: now,
        };
        let _ = self.events.send(AppEvent::Log(format!(
            "saved test progress for media #{media_id}: {position_ms}/{duration_ms} ms"
        )));
        Ok(progress)
    }

    pub fn clear(&self, media_id: i64) -> AppResult<()> {
        self.repository.clear_progress(media_id)?;
        let _ = self.events.send(AppEvent::Log(format!(
            "cleared progress for media #{media_id}"
        )));
        Ok(())
    }
}

#[derive(Clone)]
pub struct DanmakuService {
    config: Arc<ConfigStore>,
    client: Client,
    events: mpsc::Sender<AppEvent>,
}

impl DanmakuService {
    pub fn new(config: Arc<ConfigStore>, events: mpsc::Sender<AppEvent>) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("slint-bangumi/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            config,
            client,
            events,
        })
    }

    pub fn load_for_media(&self, media: &MediaItem) {
        let _ = self.events.send(AppEvent::Log(format!(
            "loading danmaku for {}",
            media.file_name
        )));

        match self.match_dandanplay(media) {
            Ok(result) => {
                let _ = self.events.send(AppEvent::DanmakuMatched(result));
            }
            Err(error) => {
                let _ = self
                    .events
                    .send(AppEvent::Log(format!("danmaku load failed: {error}")));
            }
        }
    }

    pub fn match_dandanplay(&self, media: &MediaItem) -> AppResult<DanmakuMatch> {
        if media.deleted_at.is_some() {
            return Err(AppError::MediaNotFound);
        }
        let config = self.config.snapshot().dandanplay;
        validate_dandanplay_config(&config)?;

        let match_result = self.match_episode(media, &config)?;
        let comment_count = self.fetch_comment_count(match_result.episode_id, &config)?;

        Ok(DanmakuMatch {
            provider: "dandanplay".to_string(),
            title: match_title(&match_result).unwrap_or_else(|| media.file_name.clone()),
            episode: match_result.episode_title,
            comment_count,
        })
    }

    fn match_episode(
        &self,
        media: &MediaItem,
        config: &DandanplayConfig,
    ) -> AppResult<MatchResult> {
        let path = "/api/v2/match";
        let url = format!("{DANDANPLAY_BASE_URL}{path}");
        let file_name = media
            .path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(&media.file_name)
            .to_string();

        let request = MatchRequest {
            file_name: Some(file_name),
            file_hash: media.file_hash.clone(),
            file_size: media.file_size as i64,
            video_duration: 0,
            match_mode: if media.file_hash.is_some() {
                "hashAndFileName"
            } else {
                "fileNameOnly"
            },
        };

        let response = self
            .signed_headers(self.client.post(url).json(&request), path, config)
            .send()?;
        let status = response.status();
        if !status.is_success() {
            let error = response
                .headers()
                .get("X-Error-Message")
                .and_then(|value| value.to_str().ok())
                .unwrap_or(status.as_str())
                .to_string();
            return Err(AppError::Api(format!("dandanplay match rejected: {error}")));
        }

        let response = response.json::<MatchResponse>()?;
        ensure_api_success(
            response.success,
            response.error_code,
            response.error_message,
        )?;

        let mut matches = response.matches.unwrap_or_default();
        if matches.is_empty() {
            return Err(AppError::Api("dandanplay returned no matches".to_string()));
        }

        Ok(matches.remove(0))
    }

    fn fetch_comment_count(&self, episode_id: i64, config: &DandanplayConfig) -> AppResult<usize> {
        let path = format!("/api/v2/comment/{episode_id}");
        let url = format!("{DANDANPLAY_BASE_URL}{path}");
        let response = self
            .signed_headers(
                self.client.get(url).query(&[
                    ("from", "0"),
                    ("withRelated", "true"),
                    ("chConvert", "1"),
                ]),
                &path,
                config,
            )
            .send()?;
        let status = response.status();
        if !status.is_success() {
            let error = response
                .headers()
                .get("X-Error-Message")
                .and_then(|value| value.to_str().ok())
                .unwrap_or(status.as_str())
                .to_string();
            return Err(AppError::Api(format!(
                "dandanplay comment request rejected: {error}"
            )));
        }

        let response = response.json::<CommentResponse>()?;
        Ok(response
            .comments
            .map(|comments| comments.len())
            .unwrap_or(response.count.max(0) as usize))
    }

    fn signed_headers(
        &self,
        request: RequestBuilder,
        path: &str,
        config: &DandanplayConfig,
    ) -> RequestBuilder {
        let timestamp = unix_timestamp_secs();
        request
            .header("X-AppId", config.app_id.as_str())
            .header("X-Timestamp", timestamp.to_string())
            .header(
                "X-Signature",
                dandanplay_signature(&config.app_id, timestamp, path, &config.app_secret),
            )
    }
}

const DANDANPLAY_BASE_URL: &str = "https://api.dandanplay.net";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchRequest<'a> {
    file_name: Option<String>,
    file_hash: Option<String>,
    file_size: i64,
    video_duration: i32,
    match_mode: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchResponse {
    success: bool,
    error_code: i32,
    error_message: Option<String>,
    matches: Option<Vec<MatchResult>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchResult {
    episode_id: i64,
    anime_title: Option<String>,
    episode_title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommentResponse {
    count: i32,
    comments: Option<Vec<CommentData>>,
}

#[derive(Debug, Deserialize)]
struct CommentData {
    #[allow(dead_code)]
    cid: i64,
}

fn validate_dandanplay_config(config: &DandanplayConfig) -> AppResult<()> {
    if config.app_id.trim().is_empty() || config.app_secret.trim().is_empty() {
        return Err(AppError::Config(
            "dandanplay app_id and app_secret are required".to_string(),
        ));
    }
    Ok(())
}

fn ensure_api_success(
    success: bool,
    error_code: i32,
    error_message: Option<String>,
) -> AppResult<()> {
    if success && error_code == 0 {
        return Ok(());
    }

    Err(AppError::Api(format!(
        "dandanplay error {error_code}: {}",
        error_message.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn match_title(result: &MatchResult) -> Option<String> {
    match (&result.anime_title, &result.episode_title) {
        (Some(anime), Some(episode)) if !anime.is_empty() && !episode.is_empty() => {
            Some(format!("{anime} - {episode}"))
        }
        (Some(anime), _) if !anime.is_empty() => Some(anime.clone()),
        (_, Some(episode)) if !episode.is_empty() => Some(episode.clone()),
        _ => None,
    }
}

fn dandanplay_signature(app_id: &str, timestamp: i64, path: &str, app_secret: &str) -> String {
    let data = format!("{app_id}{timestamp}{path}{app_secret}");
    BASE64_STANDARD.encode(Sha256::digest(data.as_bytes()))
}

fn unix_timestamp_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod dandanplay_tests {
    use super::*;

    #[test]
    fn signs_request_like_official_algorithm() {
        assert_eq!(
            dandanplay_signature("app", 1, "/api/v2/match", "secret"),
            "bhmxR4cp1CqSfgXiWkbRGGR1QtkhNnR7qvyB1CBFbRA="
        );
    }
}
