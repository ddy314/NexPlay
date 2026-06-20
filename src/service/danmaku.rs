use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{ConfigStore, DandanplayConfig};
use crate::domain::{DanmakuMatch, MediaItem};
use crate::error::{AppError, AppResult};
use crate::repository::Repository;
use crate::task::{self, AppEvent};

#[derive(Clone)]
pub struct DanmakuService {
    config: Arc<ConfigStore>,
    repository: Repository,
    client: Client,
    events: mpsc::Sender<AppEvent>,
}

impl DanmakuService {
    pub fn new(
        config: Arc<ConfigStore>,
        repository: Repository,
        events: mpsc::Sender<AppEvent>,
    ) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("NexPlay/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            config,
            repository,
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
                if let Err(error) = self.repository.upsert_danmaku_match(
                    media.id,
                    &result,
                    task::unix_timestamp_ms(),
                ) {
                    let _ = self
                        .events
                        .send(AppEvent::Log(format!("danmaku cache failed: {error}")));
                }
                let _ = self.events.send(AppEvent::DanmakuMatched(result));
            }
            Err(error) => {
                let _ = self
                    .events
                    .send(AppEvent::Log(format!("danmaku load failed: {error}")));
            }
        }
    }

    pub fn cached_or_match_dandanplay(&self, media: &MediaItem) -> AppResult<Option<DanmakuMatch>> {
        if let Some(result) = self.repository.danmaku_match_for_media(media.id)? {
            return Ok(Some(result));
        }

        let result = self.match_dandanplay(media)?;
        self.repository
            .upsert_danmaku_match(media.id, &result, task::unix_timestamp_ms())?;
        Ok(Some(result))
    }

    pub fn cached_dandanplay(&self, media: &MediaItem) -> AppResult<Option<DanmakuMatch>> {
        self.repository.danmaku_match_for_media(media.id)
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
            anime_id: Some(match_result.anime_id),
            episode_id: Some(match_result.episode_id),
            anime_title: match_result.anime_title.clone(),
            episode: match_result.episode_title,
            comment_count,
            exact: match_result.is_matched,
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

        let exact = response.is_matched.unwrap_or(false);
        let mut matches = response.matches.unwrap_or_default();
        if matches.is_empty() {
            return Err(AppError::Api("dandanplay returned no matches".to_string()));
        }

        let mut best = matches.remove(0);
        best.is_matched = exact;
        Ok(best)
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
    is_matched: Option<bool>,
    matches: Option<Vec<MatchResult>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchResult {
    episode_id: i64,
    anime_id: i64,
    anime_title: Option<String>,
    episode_title: Option<String>,
    #[serde(default)]
    is_matched: bool,
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
mod tests {
    use super::*;

    #[test]
    fn signs_request_like_official_algorithm() {
        assert_eq!(
            dandanplay_signature("app", 1, "/api/v2/match", "secret"),
            "bhmxR4cp1CqSfgXiWkbRGGR1QtkhNnR7qvyB1CBFbRA="
        );
    }
}
