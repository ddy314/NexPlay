use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::{Config as TsConfig, TS};

use crate::app::AppContext;
use crate::config::{AppConfig, BangumiConfig, DandanplayConfig, DatabaseConfig, LoggingConfig};
use crate::domain::{ScanSummary, UiSeriesCardData};
use crate::error::AppResult;
use crate::task::AppEvent;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BackendSnapshot {
    pub subjects: Vec<FrontendSubject>,
    pub stats: LibraryStats,
    pub settings: FrontendSettings,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub tentative: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrontendSettings {
    pub bangumi_enabled: bool,
    pub bangumi_auto_match: bool,
    pub bangumi_cache_images: bool,
    pub dandanplay_configured: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FrontendMatchStatus {
    Matched,
    Tentative,
    Unmatched,
    Failed,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrontendSubject {
    pub id: String,
    pub media_id: i64,
    pub subject_id: i64,
    pub title: String,
    pub title_cn: String,
    pub year: i32,
    pub air_date: String,
    pub rating: f64,
    pub rank: i64,
    pub tags: Vec<String>,
    pub summary: String,
    pub poster: String,
    pub hero: String,
    pub status: FrontendMatchStatus,
    pub episodes: usize,
    pub watched_episodes: usize,
    #[ts(optional)]
    pub current_episode: Option<usize>,
    pub progress: f64,
    pub files: usize,
    pub total_size: String,
    #[ts(optional)]
    pub last_played: Option<String>,
    pub new_episode: bool,
    pub metadata_ready: bool,
    pub file_summary: String,
    pub local_files: Vec<FrontendLocalFile>,
    pub episodes_detail: Vec<FrontendEpisode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLocalFile {
    pub media_id: i64,
    pub file_name: String,
    pub file_size: String,
    #[ts(optional)]
    pub episode: Option<usize>,
    pub modified_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrontendEpisode {
    pub episode: usize,
    pub title: String,
    pub title_cn: String,
    pub air_date: String,
    pub cached: bool,
    #[ts(optional)]
    pub media_id: Option<i64>,
    #[ts(optional)]
    pub file_name: Option<String>,
    #[ts(optional)]
    pub file_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrontendEditableSettings {
    pub media_libraries: Vec<String>,
    pub database_path: String,
    pub bangumi_enabled: bool,
    pub bangumi_base_url: String,
    pub bangumi_access_token: String,
    pub bangumi_user_agent: String,
    pub bangumi_request_timeout_secs: u64,
    pub bangumi_auto_match: bool,
    pub bangumi_cache_images: bool,
    pub dandanplay_app_id: String,
    pub dandanplay_app_secret: String,
    pub dandanplay_api_key: String,
    pub logging_level: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanResponse {
    pub summary: ScanSummary,
    pub scraped: usize,
    pub snapshot: BackendSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OpenMediaRequest {
    pub media_id: i64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OpenMediaResponse {
    pub opened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceRequest {
    pub media_id: i64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceResponse {
    pub media_id: i64,
    pub file_name: String,
    pub file_size: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BackendEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[ts(optional)]
    pub message: Option<String>,
    #[ts(optional)]
    pub scanned: Option<usize>,
    #[ts(optional)]
    pub indexed: Option<usize>,
    #[ts(optional)]
    pub processed: Option<usize>,
    #[ts(optional)]
    pub total: Option<usize>,
    #[ts(optional)]
    pub summary: Option<ScanSummary>,
    #[ts(optional)]
    pub media_id: Option<i64>,
    #[ts(optional)]
    pub subject_id: Option<i64>,
    #[ts(optional)]
    pub image_kind: Option<String>,
    #[ts(optional)]
    pub target_id: Option<i64>,
}

impl BackendEvent {
    fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            message: None,
            scanned: None,
            indexed: None,
            processed: None,
            total: None,
            summary: None,
            media_id: None,
            subject_id: None,
            image_kind: None,
            target_id: None,
        }
    }
}

pub fn settings_config(context: &AppContext) -> AppResult<FrontendEditableSettings> {
    Ok(frontend_settings_from_config(
        context.media.config_snapshot(),
    ))
}

pub fn save_settings_config(
    context: &AppContext,
    input: FrontendEditableSettings,
) -> AppResult<FrontendEditableSettings> {
    let saved = context
        .media
        .replace_config(config_from_frontend_settings(input))?;
    Ok(frontend_settings_from_config(saved))
}

pub fn snapshot(context: &AppContext) -> AppResult<BackendSnapshot> {
    let cards = context.media.list_series_cards()?;
    let series_count = cards.len();
    let (_, _, unmatched) = context.media.library_counts()?;
    let tentative = context.metadata.tentative_count()?;
    let flags = context.media.settings_flags();

    Ok(BackendSnapshot {
        subjects: cards
            .into_iter()
            .map(frontend_subject_from_series)
            .collect(),
        stats: LibraryStats {
            total: series_count,
            matched: series_count,
            unmatched,
            tentative,
        },
        settings: FrontendSettings {
            bangumi_enabled: flags.bangumi_enabled,
            bangumi_auto_match: flags.bangumi_auto_match,
            bangumi_cache_images: flags.bangumi_cache_images,
            dandanplay_configured: flags.dandanplay_configured,
        },
    })
}

pub fn scan(context: &AppContext) -> AppResult<ScanResponse> {
    let summary = context.media.scan_now()?;
    let scraped = context.metadata.scrape_library_blocking()?;
    let snapshot = snapshot(context)?;
    Ok(ScanResponse {
        summary,
        scraped,
        snapshot,
    })
}

pub fn open_media(context: &AppContext, input: OpenMediaRequest) -> AppResult<OpenMediaResponse> {
    context.media.open_media_by_id(input.media_id)?;
    Ok(OpenMediaResponse { opened: true })
}

pub fn media_source(
    context: &AppContext,
    input: MediaSourceRequest,
) -> AppResult<MediaSourceResponse> {
    let media = context.media.playback_media_by_id(input.media_id)?;
    Ok(MediaSourceResponse {
        media_id: media.id,
        file_name: media.file_name,
        file_size: format_bytes(media.file_size),
        source_url: normalize_asset_path(&media.path.to_string_lossy()),
    })
}

fn frontend_subject_from_series(card: UiSeriesCardData) -> FrontendSubject {
    let display_title = if card.title_cn.trim().is_empty() {
        card.title.clone()
    } else {
        card.title_cn.clone()
    };
    let progress = if card.episode_count == 0 {
        0.0
    } else {
        (card.linked_episode_count as f64 / card.episode_count as f64).clamp(0.0, 1.0)
    };
    let local_files = card
        .local_files
        .into_iter()
        .map(|file| FrontendLocalFile {
            media_id: file.media_id,
            file_name: file.file_name,
            file_size: format_bytes(file.file_size),
            episode: file.episode_number.map(|episode| episode.round() as usize),
            modified_at: file.modified_at,
        })
        .collect();
    let episodes_detail = card
        .episodes
        .into_iter()
        .map(|episode| FrontendEpisode {
            episode: rounded_episode_number(episode.episode_number),
            title: episode.title,
            title_cn: episode.title_cn,
            air_date: episode.air_date,
            cached: episode.media_id.is_some(),
            media_id: episode.media_id,
            file_name: episode.file_name,
            file_size: episode.file_size.map(format_bytes),
        })
        .collect();

    FrontendSubject {
        id: format!("subject-{}", card.subject_id),
        media_id: 0,
        subject_id: card.subject_id,
        title: display_title,
        title_cn: card.title,
        year: card
            .air_date
            .get(0..4)
            .and_then(|year| year.parse().ok())
            .unwrap_or_default(),
        air_date: card.air_date,
        rating: card.rating.unwrap_or_default(),
        rank: card.rank.unwrap_or_default(),
        tags: card.tags,
        summary: card.summary,
        poster: normalize_asset_path(&card.poster_path),
        hero: normalize_asset_path(&card.hero_path),
        status: FrontendMatchStatus::Matched,
        episodes: card.episode_count,
        watched_episodes: card.linked_episode_count,
        current_episode: None,
        progress,
        files: card.file_count,
        total_size: format_bytes(card.total_size),
        last_played: None,
        new_episode: false,
        metadata_ready: true,
        file_summary: card.latest_file_name,
        local_files,
        episodes_detail,
    }
}

pub fn frontend_event_from_app(event: AppEvent) -> BackendEvent {
    match event {
        AppEvent::Log(message) => BackendEvent {
            message: Some(message),
            ..BackendEvent::new("log")
        },
        AppEvent::ScanStarted => BackendEvent {
            message: Some("扫描已开始".to_string()),
            ..BackendEvent::new("scanStarted")
        },
        AppEvent::ScanProgress { scanned, indexed } => BackendEvent {
            scanned: Some(scanned),
            indexed: Some(indexed),
            message: Some(format!("已扫描 {scanned} 个文件")),
            ..BackendEvent::new("scanProgress")
        },
        AppEvent::ScanFinished { summary, .. } => BackendEvent {
            message: Some(format!("文件扫描完成：{} 个文件", summary.scanned_files)),
            summary: Some(summary),
            ..BackendEvent::new("scanFinished")
        },
        AppEvent::ScanFailed(error) => BackendEvent {
            message: Some(error),
            ..BackendEvent::new("scanFailed")
        },
        AppEvent::DanmakuMatched(match_result) => BackendEvent {
            message: Some(match_result.title),
            ..BackendEvent::new("danmakuMatched")
        },
        AppEvent::MetadataMatchStarted { media_id } => BackendEvent {
            media_id: Some(media_id),
            ..BackendEvent::new("metadataStarted")
        },
        AppEvent::MetadataMatchProgress { processed, total } => BackendEvent {
            processed: Some(processed),
            total: Some(total),
            message: Some(format!("元数据整理 {processed}/{total}")),
            ..BackendEvent::new("metadataProgress")
        },
        AppEvent::MetadataMatchFinished {
            media_id,
            subject_id,
            title,
        } => BackendEvent {
            media_id: Some(media_id),
            subject_id,
            message: Some(title.unwrap_or_else(|| format!("media #{media_id}"))),
            ..BackendEvent::new("metadataFinished")
        },
        AppEvent::SubjectUpdated { subject_id } => BackendEvent {
            subject_id: Some(subject_id),
            ..BackendEvent::new("subjectUpdated")
        },
        AppEvent::ImageCached {
            subject_id,
            image_kind,
        } => BackendEvent {
            subject_id: Some(subject_id),
            image_kind: Some(image_kind),
            ..BackendEvent::new("imageCached")
        },
        AppEvent::MetadataFailed { target_id, error } => BackendEvent {
            target_id: Some(target_id),
            message: Some(error),
            ..BackendEvent::new("metadataFailed")
        },
        AppEvent::MetadataStatus(message) => BackendEvent {
            message: Some(message),
            ..BackendEvent::new("metadataStatus")
        },
    }
}

pub fn export_types(output_path: impl AsRef<Path>) -> AppResult<()> {
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| crate::error::io_error(parent, error))?;
    }

    let ts_config = TsConfig::default().with_large_int("number");
    let mut declarations = [
        ScanSummary::decl(&ts_config),
        LibraryStats::decl(&ts_config),
        FrontendSettings::decl(&ts_config),
        FrontendMatchStatus::decl(&ts_config),
        FrontendLocalFile::decl(&ts_config),
        FrontendEpisode::decl(&ts_config),
        FrontendSubject::decl(&ts_config),
        BackendSnapshot::decl(&ts_config),
        FrontendEditableSettings::decl(&ts_config),
        ScanResponse::decl(&ts_config),
        OpenMediaRequest::decl(&ts_config),
        OpenMediaResponse::decl(&ts_config),
        MediaSourceRequest::decl(&ts_config),
        MediaSourceResponse::decl(&ts_config),
        BackendEvent::decl(&ts_config),
    ]
    .join("\n\n")
    .replace("\ntype ", "\nexport type ");
    if declarations.starts_with("type ") {
        declarations = format!("export {declarations}");
    }

    let content = format!(
        "/* eslint-disable */\n// This file is generated by `cargo run --quiet -- export-types`.\n\n{declarations}\n"
    );
    std::fs::write(output_path, content)
        .map_err(|error| crate::error::io_error(output_path, error))?;
    Ok(())
}

fn frontend_settings_from_config(config: AppConfig) -> FrontendEditableSettings {
    FrontendEditableSettings {
        media_libraries: config
            .media_libraries
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        database_path: config.database.path.display().to_string(),
        bangumi_enabled: config.bangumi.enabled,
        bangumi_base_url: config.bangumi.base_url,
        bangumi_access_token: config.bangumi.access_token,
        bangumi_user_agent: config.bangumi.user_agent,
        bangumi_request_timeout_secs: config.bangumi.request_timeout_secs,
        bangumi_auto_match: config.bangumi.auto_match,
        bangumi_cache_images: config.bangumi.cache_images,
        dandanplay_app_id: config.dandanplay.app_id,
        dandanplay_app_secret: config.dandanplay.app_secret,
        dandanplay_api_key: config.dandanplay.api_key,
        logging_level: config.logging.level,
    }
}

fn config_from_frontend_settings(input: FrontendEditableSettings) -> AppConfig {
    AppConfig {
        database: DatabaseConfig {
            path: PathBuf::from(input.database_path.trim()),
        },
        media_libraries: input
            .media_libraries
            .into_iter()
            .map(|path| PathBuf::from(path.trim()))
            .filter(|path| !path.as_os_str().is_empty())
            .collect(),
        dandanplay: DandanplayConfig {
            app_id: input.dandanplay_app_id,
            app_secret: input.dandanplay_app_secret,
            api_key: input.dandanplay_api_key,
        },
        bangumi: BangumiConfig {
            enabled: input.bangumi_enabled,
            base_url: input.bangumi_base_url,
            access_token: input.bangumi_access_token,
            user_agent: input.bangumi_user_agent,
            request_timeout_secs: input.bangumi_request_timeout_secs.max(1),
            auto_match: input.bangumi_auto_match,
            cache_images: input.bangumi_cache_images,
        },
        logging: LoggingConfig {
            level: input.logging_level,
        },
    }
}

fn rounded_episode_number(value: f64) -> usize {
    value.round().max(0.0) as usize
}

fn format_bytes(value: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if value as f64 >= GIB {
        format!("{:.1} GB", value as f64 / GIB)
    } else if value as f64 >= MIB {
        format!("{:.1} MB", value as f64 / MIB)
    } else {
        format!("{value} B")
    }
}

fn normalize_asset_path(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else if path.starts_with("file://")
        || path.starts_with("http://")
        || path.starts_with("https://")
    {
        path.to_string()
    } else {
        let asset_path = Path::new(path);
        let absolute = if asset_path.is_absolute() {
            asset_path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| Path::new(".").to_path_buf())
                .join(asset_path)
        };
        format!("file://{}", absolute.display())
    }
}
