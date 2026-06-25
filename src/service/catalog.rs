use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use reqwest::blocking::{Client, multipart};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::config::{ConfigStore, DandanplayConfig, QbittorrentConfig};
use crate::domain::{DownloadTask, ResourceCandidate};
use crate::error::{AppError, AppResult};
use crate::metadata::bangumi::BangumiProvider;
use crate::metadata::provider::{MetadataProvider, SubjectSearchResult};
use crate::repository::Repository;
use crate::task::{self, AppEvent};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSubjectData {
    pub id: String,
    pub provider: String,
    pub provider_subject_id: String,
    pub source: String,
    pub title: String,
    pub title_cn: String,
    pub summary: String,
    pub air_date: String,
    pub rating: f64,
    pub rank: i64,
    pub poster: String,
    pub hero: String,
    pub episodes: usize,
    pub files: usize,
    pub local: bool,
    pub metadata_ready: bool,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeResourceData {
    pub id: i64,
    pub provider: String,
    pub title: String,
    pub subtitle_group: String,
    pub resolution: String,
    pub torrent_url: String,
    pub page_url: String,
    pub info_hash: String,
    pub size: String,
    pub seeders: i64,
    pub leechers: i64,
    pub downloads: i64,
    pub trusted: bool,
    pub remake: bool,
    pub batch: bool,
    pub episode_start: i64,
    pub episode_end: i64,
    pub published_at: String,
    pub score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskData {
    pub id: i64,
    #[ts(optional)]
    pub resource_id: Option<i64>,
    pub subject_provider: String,
    pub provider_subject_id: String,
    #[ts(optional)]
    pub episode_number: Option<f64>,
    pub title: String,
    pub torrent_url: String,
    pub info_hash: String,
    pub qbittorrent_hash: String,
    pub status: String,
    pub progress: f64,
    pub save_path: String,
    pub error: String,
    pub updated_at: i64,
}

#[derive(Clone)]
pub struct CatalogService {
    config: Arc<ConfigStore>,
    repository: Repository,
    client: Client,
    events: mpsc::Sender<AppEvent>,
}

impl CatalogService {
    pub fn new(
        config: Arc<ConfigStore>,
        repository: Repository,
        events: mpsc::Sender<AppEvent>,
    ) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("NexPlay/", env!("CARGO_PKG_VERSION")))
            .cookie_store(true)
            .build()?;
        Ok(Self {
            config,
            repository,
            client,
            events,
        })
    }

    pub fn search_catalog(&self, query: &str, limit: usize) -> AppResult<Vec<CatalogSubjectData>> {
        let query = query.trim();
        if query.chars().count() < 2 {
            return Ok(Vec::new());
        }

        let variants = search_query_variants(query);
        let (bangumi_tx, bangumi_rx) = std::sync::mpsc::channel();
        let (dandan_tx, dandan_rx) = std::sync::mpsc::channel();
        let (anilist_tx, anilist_rx) = std::sync::mpsc::channel();

        if let Ok(provider) = self.bangumi_provider() {
            let variants = variants.clone();
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for variant in variants.iter().take(6) {
                    match provider.search_subjects(variant) {
                        Ok(subjects) => out.extend(
                            subjects
                                .into_iter()
                                .take(8)
                                .map(catalog_from_bangumi_search),
                        ),
                        Err(error) => {
                            let _ = bangumi_tx.send(Err(error));
                            return;
                        }
                    }
                }
                let _ = bangumi_tx.send(Ok(out));
            });
        } else {
            let _ = bangumi_tx.send(Ok(Vec::new()));
        }

        if self.dandanplay_configured() {
            let service = self.clone();
            let variants = variants.clone();
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for variant in variants.iter().take(3) {
                    match service.search_dandanplay_anime(variant) {
                        Ok(subjects) => out.extend(subjects.into_iter().take(8)),
                        Err(error) => {
                            let _ = dandan_tx.send(Err(error));
                            return;
                        }
                    }
                }
                let _ = dandan_tx.send(Ok(out));
            });
        } else {
            let _ = dandan_tx.send(Ok(Vec::new()));
        }

        {
            let service = self.clone();
            let variants = variants.clone();
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for variant in variants.iter().take(4) {
                    match service.search_anilist(variant) {
                        Ok(subjects) => out.extend(subjects.into_iter().take(8)),
                        Err(error) => {
                            let _ = anilist_tx.send(Err(error));
                            return;
                        }
                    }
                }
                let _ = anilist_tx.send(Ok(out));
            });
        }

        let mut results = Vec::new();
        let mut seen = HashSet::new();
        let deadline = Instant::now() + Duration::from_secs(8);
        for receiver in [bangumi_rx, dandan_rx, anilist_rx] {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| Duration::from_millis(1));
            let received = receiver.recv_timeout(remaining);
            match received {
                Ok(Ok(subjects)) => {
                    for subject in subjects.into_iter().take(limit.max(1)) {
                        push_unique(&mut results, &mut seen, subject);
                    }
                }
                Ok(Err(error)) => self.send_log(format!("online catalog search failed: {error}")),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => self.send_log(
                    "online catalog search timed out; returning partial results".to_string(),
                ),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.send_log("online catalog worker disconnected".to_string())
                }
            }
        }

        let mut results = merge_equivalent_subjects(results);
        results.sort_by(|left, right| {
            online_subject_score(right, query)
                .cmp(&online_subject_score(left, query))
                .then_with(|| left.title.cmp(&right.title))
        });
        results.truncate(limit.max(1));
        Ok(results)
    }

    pub fn online_subject(
        &self,
        provider: &str,
        provider_subject_id: &str,
    ) -> AppResult<CatalogSubjectData> {
        match provider {
            "bangumi" => {
                let provider = self.bangumi_provider()?;
                let detail = provider.get_subject(provider_subject_id)?;
                let episodes = provider
                    .get_episodes(provider_subject_id)
                    .unwrap_or_default();
                Ok(CatalogSubjectData {
                    id: format!("online-bangumi-{provider_subject_id}"),
                    provider: detail.provider,
                    provider_subject_id: detail.provider_subject_id,
                    source: "bangumi".to_string(),
                    title: detail.title,
                    title_cn: detail.title_cn.unwrap_or_default(),
                    summary: detail.summary.unwrap_or_default(),
                    air_date: detail.air_date.unwrap_or_default(),
                    rating: detail.rating.unwrap_or_default(),
                    rank: detail.rank.unwrap_or_default(),
                    poster: detail.images.large.unwrap_or_default(),
                    hero: detail.images.common.unwrap_or_default(),
                    episodes: episodes.len().max(detail.episode_count.unwrap_or_default()),
                    files: 0,
                    local: false,
                    metadata_ready: true,
                    tags: detail.tags,
                    aliases: detail.aliases,
                })
            }
            "dandanplay" => self.dandanplay_subject(provider_subject_id).or_else(|_| {
                Ok(CatalogSubjectData {
                    id: format!("online-dandanplay-{provider_subject_id}"),
                    provider: "dandanplay".to_string(),
                    provider_subject_id: provider_subject_id.to_string(),
                    source: "dandanplay".to_string(),
                    title: provider_subject_id.to_string(),
                    title_cn: String::new(),
                    summary: String::new(),
                    air_date: String::new(),
                    rating: 0.0,
                    rank: 0,
                    poster: String::new(),
                    hero: String::new(),
                    episodes: 0,
                    files: 0,
                    local: false,
                    metadata_ready: false,
                    tags: Vec::new(),
                    aliases: Vec::new(),
                })
            }),
            "anilist" => self.anilist_subject(provider_subject_id),
            other => Err(AppError::Api(format!(
                "unsupported online provider: {other}"
            ))),
        }
    }

    pub fn search_episode_resources(
        &self,
        subject_provider: &str,
        provider_subject_id: &str,
        title: &str,
        title_cn: &str,
        aliases: &[String],
        episode_number: f64,
        limit: usize,
    ) -> AppResult<Vec<EpisodeResourceData>> {
        let config = self.config.snapshot().nyaa;
        if !config.enabled {
            return Err(AppError::Config(
                "Nyaa resource search is disabled".to_string(),
            ));
        }

        let mut resources = Vec::new();
        let mut seen = HashSet::new();
        let title_candidates = resource_title_candidates(title, title_cn, aliases);
        if title_candidates.is_empty() {
            return Ok(Vec::new());
        }

        for query in resource_queries(&title_candidates, episode_number) {
            let mut candidates = self.fetch_nyaa_rss(&config.base_url, &config.category, &query)?;
            for candidate in candidates.drain(..) {
                if !seen.insert(candidate.torrent_url.clone()) {
                    continue;
                }
                if !resource_matches_title(&candidate.title, &title_candidates) {
                    continue;
                }
                if !resource_matches_episode(&candidate.title, episode_number) {
                    continue;
                }
                let mut candidate = candidate;
                candidate.subject_provider = subject_provider.to_string();
                candidate.provider_subject_id = provider_subject_id.to_string();
                candidate.episode_number = Some(episode_number);
                let id = self
                    .repository
                    .upsert_resource_candidate(&candidate, task::unix_timestamp_ms())?;
                candidate.id = id;
                resources.push(frontend_resource_from_domain(candidate, episode_number));
            }
            if resources.len() >= limit.max(1) {
                break;
            }
        }

        resources.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| right.seeders.cmp(&left.seeders))
                .then_with(|| left.title.cmp(&right.title))
        });
        resources.truncate(limit.max(1));
        Ok(resources)
    }

    pub fn start_resource_download(
        &self,
        resource: &EpisodeResourceData,
        subject_provider: &str,
        provider_subject_id: &str,
        episode_number: Option<f64>,
    ) -> AppResult<DownloadTaskData> {
        let now = task::unix_timestamp_ms();
        let task = DownloadTask {
            id: 0,
            resource_id: Some(resource.id).filter(|id| *id > 0),
            subject_provider: subject_provider.to_string(),
            provider_subject_id: provider_subject_id.to_string(),
            episode_number,
            title: resource.title.clone(),
            torrent_url: resource.torrent_url.clone(),
            info_hash: non_empty(resource.info_hash.clone()),
            qbittorrent_hash: None,
            status: "pending".to_string(),
            progress: 0.0,
            save_path: None,
            error: None,
            updated_at: now,
        };
        let task = self.repository.create_download_task(&task, now)?;

        match self.add_qbittorrent_torrent(resource) {
            Ok(qbit) => {
                self.repository.update_download_task_status(
                    task.id,
                    "queued",
                    qbit.progress,
                    qbit.hash.as_deref().or(task.info_hash.as_deref()),
                    qbit.save_path.as_deref(),
                    None,
                    task::unix_timestamp_ms(),
                )?;
            }
            Err(error) => {
                self.repository.update_download_task_status(
                    task.id,
                    "failed",
                    0.0,
                    None,
                    None,
                    Some(&error.to_string()),
                    task::unix_timestamp_ms(),
                )?;
                return self
                    .repository
                    .get_download_task(task.id)?
                    .map(frontend_download_task_from_domain)
                    .ok_or_else(|| AppError::Api("download task disappeared".to_string()));
            }
        }

        self.repository
            .get_download_task(task.id)?
            .map(frontend_download_task_from_domain)
            .ok_or_else(|| AppError::Api("download task disappeared".to_string()))
    }

    pub fn list_download_tasks(&self) -> AppResult<Vec<DownloadTaskData>> {
        Ok(self
            .repository
            .list_download_tasks()?
            .into_iter()
            .map(frontend_download_task_from_domain)
            .collect())
    }

    pub fn test_qbittorrent_connection(&self) -> AppResult<()> {
        self.login_qbittorrent().map(|_| ())
    }

    fn bangumi_provider(&self) -> AppResult<BangumiProvider> {
        BangumiProvider::new(self.config.snapshot().bangumi)
    }

    fn dandanplay_configured(&self) -> bool {
        let config = self.config.snapshot().dandanplay;
        !config.app_id.trim().is_empty() && !config.app_secret.trim().is_empty()
    }

    fn search_dandanplay_anime(&self, query: &str) -> AppResult<Vec<CatalogSubjectData>> {
        let config = self.config.snapshot().dandanplay;
        let path = "/api/v2/search/anime";
        let response = self
            .signed_dandanplay_headers(
                self.client
                    .get(format!("{DANDANPLAY_BASE_URL}{path}"))
                    .query(&[("keyword", query)]),
                path,
                &config,
            )
            .send()?;
        ensure_http_success(response.status(), "dandanplay anime search")?;
        let response = response.json::<DandanSearchAnimeResponse>()?;
        ensure_dandanplay_success(
            response.success,
            response.error_code,
            response.error_message,
        )?;
        Ok(response
            .animes
            .unwrap_or_default()
            .into_iter()
            .map(catalog_from_dandanplay_search)
            .collect())
    }

    fn dandanplay_subject(&self, provider_subject_id: &str) -> AppResult<CatalogSubjectData> {
        let config = self.config.snapshot().dandanplay;
        let path = format!("/api/v2/bangumi/{provider_subject_id}");
        let response = self
            .signed_dandanplay_headers(
                self.client.get(format!("{DANDANPLAY_BASE_URL}{path}")),
                &path,
                &config,
            )
            .send()?;
        ensure_http_success(response.status(), "dandanplay bangumi detail")?;
        let response = response.json::<DandanBangumiDetailResponse>()?;
        ensure_dandanplay_success(
            response.success,
            response.error_code,
            response.error_message,
        )?;
        let Some(bangumi) = response.bangumi else {
            return Err(AppError::Api(
                "dandanplay detail returned no bangumi".to_string(),
            ));
        };
        Ok(catalog_from_dandanplay_detail(bangumi))
    }

    fn search_anilist(&self, query: &str) -> AppResult<Vec<CatalogSubjectData>> {
        let response = self
            .client
            .post("https://graphql.anilist.co")
            .json(&serde_json::json!({
                "query": "query ($search: String) { Page(page: 1, perPage: 8) { media(search: $search, type: ANIME) { id title { romaji english native } synonyms description startDate { year month day } episodes coverImage { large extraLarge } averageScore popularity } } }",
                "variables": { "search": query }
            }))
            .send()?;
        ensure_http_success(response.status(), "AniList search")?;
        let response = response.json::<AniListSearchResponse>()?;
        Ok(response
            .data
            .page
            .media
            .into_iter()
            .map(catalog_from_anilist)
            .collect())
    }

    fn anilist_subject(&self, provider_subject_id: &str) -> AppResult<CatalogSubjectData> {
        let id = provider_subject_id.parse::<i64>().map_err(|_| {
            AppError::Api(format!("invalid AniList subject id: {provider_subject_id}"))
        })?;
        let response = self
            .client
            .post("https://graphql.anilist.co")
            .json(&serde_json::json!({
                "query": "query ($id: Int) { Media(id: $id, type: ANIME) { id title { romaji english native } synonyms description startDate { year month day } episodes coverImage { large extraLarge } averageScore popularity } }",
                "variables": { "id": id }
            }))
            .send()?;
        ensure_http_success(response.status(), "AniList detail")?;
        let response = response.json::<AniListDetailResponse>()?;
        response
            .data
            .media
            .map(catalog_from_anilist)
            .ok_or_else(|| AppError::Api("AniList detail returned no media".to_string()))
    }

    fn fetch_nyaa_rss(
        &self,
        base_url: &str,
        category: &str,
        query: &str,
    ) -> AppResult<Vec<ResourceCandidate>> {
        let response = self
            .client
            .get(format!("{}/", base_url.trim_end_matches('/')))
            .query(&[("page", "rss"), ("q", query), ("c", category), ("f", "0")])
            .send()?;
        ensure_http_success(response.status(), "Nyaa RSS search")?;
        parse_nyaa_rss(&response.text()?)
    }

    fn login_qbittorrent(&self) -> AppResult<QbittorrentConfig> {
        let config = self.config.snapshot().qbittorrent;
        if !config.enabled {
            return Err(AppError::Config(
                "qBittorrent integration is disabled".to_string(),
            ));
        }
        let base = config.base_url.trim_end_matches('/');
        let response = self
            .client
            .post(format!("{base}/api/v2/auth/login"))
            .header("Referer", base)
            .form(&[
                ("username", config.username.as_str()),
                ("password", config.password.as_str()),
            ])
            .send()?;
        ensure_http_success(response.status(), "qBittorrent login")?;
        let body = response.text().unwrap_or_default();
        if !body.trim().eq_ignore_ascii_case("Ok.") {
            let message = if body.trim().eq_ignore_ascii_case("Fails.") {
                "qBittorrent login rejected username or password".to_string()
            } else {
                format!("qBittorrent login failed: {body}")
            };
            return Err(AppError::Api(message));
        }
        Ok(config)
    }

    fn add_qbittorrent_torrent(
        &self,
        resource: &EpisodeResourceData,
    ) -> AppResult<QbitTorrentInfo> {
        let config = self.login_qbittorrent()?;
        let base = config.base_url.trim_end_matches('/');
        let mut form = multipart::Form::new().text("urls", resource.torrent_url.clone());
        if !config.save_path.trim().is_empty() {
            form = form.text("savepath", config.save_path.clone());
        }
        if !config.category.trim().is_empty() {
            form = form.text("category", config.category.clone());
        }
        if !config.tags.trim().is_empty() {
            form = form.text("tags", config.tags.clone());
        }

        let response = self
            .client
            .post(format!("{base}/api/v2/torrents/add"))
            .multipart(form)
            .send()?;
        ensure_http_success(response.status(), "qBittorrent add torrent")?;

        let info_hash = resource.info_hash.to_ascii_lowercase();
        if info_hash.is_empty() {
            return Ok(QbitTorrentInfo::default());
        }
        let response = self
            .client
            .get(format!("{base}/api/v2/torrents/info"))
            .query(&[("hashes", info_hash.as_str())])
            .send()?;
        ensure_http_success(response.status(), "qBittorrent torrent info")?;
        let mut info = response.json::<Vec<QbitTorrentInfo>>()?;
        Ok(info.pop().unwrap_or_else(|| QbitTorrentInfo {
            hash: Some(info_hash),
            ..QbitTorrentInfo::default()
        }))
    }

    fn signed_dandanplay_headers(
        &self,
        request: reqwest::blocking::RequestBuilder,
        path: &str,
        config: &DandanplayConfig,
    ) -> reqwest::blocking::RequestBuilder {
        let timestamp = unix_timestamp_secs();
        request
            .header("X-AppId", config.app_id.as_str())
            .header("X-Timestamp", timestamp.to_string())
            .header(
                "X-Signature",
                dandanplay_signature(&config.app_id, timestamp, path, &config.app_secret),
            )
    }

    fn send_log(&self, message: String) {
        let _ = self.events.send(AppEvent::Log(message));
    }
}

fn push_unique(
    results: &mut Vec<CatalogSubjectData>,
    seen: &mut HashSet<String>,
    subject: CatalogSubjectData,
) {
    let key = format!("{}:{}", subject.provider, subject.provider_subject_id);
    if seen.insert(key) {
        results.push(subject);
    }
}

fn merge_equivalent_subjects(subjects: Vec<CatalogSubjectData>) -> Vec<CatalogSubjectData> {
    let mut merged: Vec<CatalogSubjectData> = Vec::new();
    for subject in subjects {
        let keys = subject_identity_keys(&subject);
        if keys.is_empty() {
            merged.push(subject);
            continue;
        }

        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| identity_sets_overlap(&subject_identity_keys(existing), &keys))
        {
            merge_subject_data(existing, subject);
        } else {
            merged.push(subject);
        }
    }
    merged
}

fn subject_identity_keys(subject: &CatalogSubjectData) -> HashSet<String> {
    let mut keys = HashSet::new();
    for value in subject
        .aliases
        .iter()
        .map(String::as_str)
        .chain(subject.tags.iter().map(String::as_str))
        .chain([subject.title.as_str(), subject.title_cn.as_str()])
    {
        if let Some(key) = normalized_identity_key(value) {
            keys.insert(key);
        }
    }
    keys
}

fn normalized_identity_key(value: &str) -> Option<String> {
    let normalized = normalize_search_text(value);
    let length = normalized.chars().count();
    if length >= 4 { Some(normalized) } else { None }
}

fn identity_sets_overlap(left: &HashSet<String>, right: &HashSet<String>) -> bool {
    left.iter().any(|key| right.contains(key))
}

fn merge_subject_data(existing: &mut CatalogSubjectData, incoming: CatalogSubjectData) {
    if provider_priority(&incoming.provider) > provider_priority(&existing.provider) {
        let mut primary = incoming;
        merge_subject_data_into(&mut primary, existing.clone());
        *existing = primary;
    } else {
        merge_subject_data_into(existing, incoming);
    }
}

fn merge_subject_data_into(primary: &mut CatalogSubjectData, secondary: CatalogSubjectData) {
    if primary.title.trim().is_empty() {
        primary.title = secondary.title.clone();
    }
    if primary.title_cn.trim().is_empty() {
        primary.title_cn = secondary.title_cn.clone();
    }
    if primary.summary.trim().is_empty() {
        primary.summary = secondary.summary.clone();
    }
    if primary.air_date.trim().is_empty() {
        primary.air_date = secondary.air_date.clone();
    }
    if primary.rating == 0.0 {
        primary.rating = secondary.rating;
    }
    if primary.rank == 0 {
        primary.rank = secondary.rank;
    }
    if primary.poster.trim().is_empty() {
        primary.poster = secondary.poster.clone();
    }
    if primary.hero.trim().is_empty() {
        primary.hero = secondary.hero.clone();
    }
    if primary.episodes == 0 {
        primary.episodes = secondary.episodes;
    }
    if !secondary.title.trim().is_empty() {
        primary.aliases.push(secondary.title);
    }
    if !secondary.title_cn.trim().is_empty() {
        primary.aliases.push(secondary.title_cn);
    }
    primary.aliases.extend(secondary.aliases);
    primary.tags.extend(secondary.tags);
    primary.aliases = dedupe_strings(primary.aliases.clone());
    primary.tags = dedupe_strings(primary.tags.clone());
}

fn provider_priority(provider: &str) -> i64 {
    match provider {
        "bangumi" => 3,
        "dandanplay" => 2,
        "anilist" => 1,
        _ => 0,
    }
}

fn catalog_from_bangumi_search(subject: SubjectSearchResult) -> CatalogSubjectData {
    CatalogSubjectData {
        id: format!("online-bangumi-{}", subject.provider_subject_id),
        provider: subject.provider,
        provider_subject_id: subject.provider_subject_id,
        source: "bangumi".to_string(),
        title: subject.title,
        title_cn: subject.title_cn.unwrap_or_default(),
        summary: subject.summary.unwrap_or_default(),
        air_date: subject.air_date.unwrap_or_default(),
        rating: subject.rating.unwrap_or_default(),
        rank: subject.rank.unwrap_or_default(),
        poster: subject.image_large.unwrap_or_default(),
        hero: subject.image_common.unwrap_or_default(),
        episodes: subject.episode_count.unwrap_or_default(),
        files: 0,
        local: false,
        metadata_ready: true,
        tags: subject.aliases.clone(),
        aliases: subject.aliases,
    }
}

fn catalog_from_dandanplay_search(subject: DandanSearchAnimeDetails) -> CatalogSubjectData {
    let anime_id = subject.anime_id.unwrap_or_default().to_string();
    CatalogSubjectData {
        id: format!("online-dandanplay-{anime_id}"),
        provider: "dandanplay".to_string(),
        provider_subject_id: anime_id,
        source: "dandanplay".to_string(),
        title: subject.anime_title.unwrap_or_default(),
        title_cn: String::new(),
        summary: String::new(),
        air_date: subject.start_date.unwrap_or_default(),
        rating: subject.rating.unwrap_or_default(),
        rank: 0,
        poster: subject.image_url.unwrap_or_default(),
        hero: String::new(),
        episodes: subject.episode_count.unwrap_or_default().max(0) as usize,
        files: 0,
        local: false,
        metadata_ready: true,
        tags: Vec::new(),
        aliases: Vec::new(),
    }
}

fn catalog_from_dandanplay_detail(subject: DandanBangumiDetail) -> CatalogSubjectData {
    let anime_id = subject.anime_id.unwrap_or_default().to_string();
    CatalogSubjectData {
        id: format!("online-dandanplay-{anime_id}"),
        provider: "dandanplay".to_string(),
        provider_subject_id: anime_id,
        source: "dandanplay".to_string(),
        title: subject.anime_title.unwrap_or_default(),
        title_cn: String::new(),
        summary: subject.introduction.unwrap_or_default(),
        air_date: subject.start_date.unwrap_or_default(),
        rating: subject.rating.unwrap_or_default(),
        rank: 0,
        poster: subject.image_url.unwrap_or_default(),
        hero: String::new(),
        episodes: subject
            .episodes
            .unwrap_or_default()
            .len()
            .max(subject.episode_count.unwrap_or_default().max(0) as usize),
        files: 0,
        local: false,
        metadata_ready: true,
        tags: Vec::new(),
        aliases: Vec::new(),
    }
}

fn catalog_from_anilist(subject: AniListMedia) -> CatalogSubjectData {
    let aliases = subject.aliases();
    let poster = subject
        .cover_image
        .as_ref()
        .and_then(|image| image.extra_large.clone().or_else(|| image.large.clone()))
        .unwrap_or_default();
    CatalogSubjectData {
        id: format!("online-anilist-{}", subject.id),
        provider: "anilist".to_string(),
        provider_subject_id: subject.id.to_string(),
        source: "anilist".to_string(),
        title: subject.title.romaji.clone().unwrap_or_else(|| {
            subject
                .title
                .english
                .clone()
                .or(subject.title.native.clone())
                .unwrap_or_default()
        }),
        title_cn: subject
            .title
            .native
            .clone()
            .or(subject.title.english.clone())
            .unwrap_or_default(),
        summary: subject
            .description
            .as_deref()
            .map(strip_html)
            .unwrap_or_default(),
        air_date: subject
            .start_date
            .as_ref()
            .and_then(AniListDate::to_iso_date)
            .unwrap_or_default(),
        rating: subject.average_score.unwrap_or_default() as f64 / 10.0,
        rank: 0,
        poster: poster.clone(),
        hero: poster,
        episodes: subject.episodes.unwrap_or_default().max(0) as usize,
        files: 0,
        local: false,
        metadata_ready: true,
        tags: aliases.clone(),
        aliases,
    }
}

fn online_subject_score(subject: &CatalogSubjectData, query: &str) -> i64 {
    let rating = (subject.rating * 10.0).round() as i64;
    let rank = if subject.rank > 0 {
        10_000 - subject.rank.min(10_000)
    } else {
        0
    };
    let provider_boost = match subject.provider.as_str() {
        "bangumi" => 50,
        "anilist" => 35,
        _ => 0,
    };
    rating + rank + provider_boost + subject_query_relevance(subject, query)
}

fn search_query_variants(query: &str) -> Vec<String> {
    let query = query.trim();
    let mut variants = vec![query.to_string()];
    let compact = query.replace([' ', '　'], "");
    if compact != query && !compact.is_empty() {
        variants.push(compact.clone());
    }

    for token in cjk_keyword_tokens(query) {
        variants.push(token.clone());
        push_cjk_search_windows(&mut variants, &token);
    }
    dedupe_strings(variants)
}

fn cjk_keyword_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in query.chars() {
        if is_cjk_character(character) && !is_cjk_separator(character) {
            current.push(character);
            continue;
        }
        push_cjk_token(&mut tokens, &current);
        current.clear();
    }
    push_cjk_token(&mut tokens, &current);
    dedupe_strings(tokens)
}

fn push_cjk_token(tokens: &mut Vec<String>, raw: &str) {
    let trimmed = trim_cjk_particles(raw);
    if trimmed.chars().count() >= 2 {
        tokens.push(trimmed);
    }
}

fn trim_cjk_particles(value: &str) -> String {
    let particles = [
        '的', '之', '在', '于', '与', '和', '及', '到', '从', '对', '把', '被', '给', '将', '为',
        '以',
    ];
    value
        .trim_matches(|character| particles.contains(&character))
        .to_string()
}

fn push_cjk_search_windows(variants: &mut Vec<String>, token: &str) {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 4 {
        return;
    }
    let mut pushed = 0usize;
    for width in 4..=chars.len().min(8) {
        variants.push(chars[..width].iter().collect());
        pushed += 1;
        if pushed >= 3 {
            break;
        }
    }
    for start in 1..chars.len() {
        for width in 4..=6 {
            if start + width > chars.len() {
                continue;
            }
            variants.push(chars[start..start + width].iter().collect());
            pushed += 1;
            if pushed >= 12 {
                return;
            }
        }
    }
}

fn is_cjk_character(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x9fff | 0x3040..=0x30ff | 0xf900..=0xfaff
    )
}

fn is_cjk_separator(character: char) -> bool {
    matches!(
        character,
        '的' | '之' | '、' | '，' | '。' | '：' | '；' | '！' | '？' | '「' | '」' | '『' | '』'
    )
}

fn subject_query_relevance(subject: &CatalogSubjectData, query: &str) -> i64 {
    let query = normalize_search_text(query);
    if query.is_empty() {
        return 0;
    }

    let mut fields = vec![subject.title.as_str(), subject.title_cn.as_str()];
    fields.extend(subject.aliases.iter().map(String::as_str));
    fields.extend(subject.tags.iter().map(String::as_str));

    let mut best = 0i64;
    for field in fields {
        let normalized = normalize_search_text(field);
        if normalized.is_empty() {
            continue;
        }
        if normalized == query {
            best = best.max(10_000);
        } else if normalized.contains(&query) || query.contains(&normalized) {
            best = best.max(6_000);
        } else {
            best = best.max((longest_common_char_run(&query, &normalized) as i64) * 450);
        }
    }
    best
}

fn normalize_search_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !character.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn longest_common_char_run(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut previous = vec![0usize; right.len() + 1];
    let mut best = 0usize;
    for left_char in &left {
        let mut current = vec![0usize; right.len() + 1];
        for (index, right_char) in right.iter().enumerate() {
            if left_char == right_char {
                current[index + 1] = previous[index] + 1;
                best = best.max(current[index + 1]);
            }
        }
        previous = current;
    }
    best
}

fn resource_title_candidates(title: &str, title_cn: &str, aliases: &[String]) -> Vec<String> {
    let mut titles = Vec::new();
    push_resource_title(&mut titles, title);
    push_resource_title(&mut titles, title_cn);
    for alias in aliases {
        push_resource_title(&mut titles, alias);
    }
    dedupe_strings(titles)
}

fn push_resource_title(titles: &mut Vec<String>, title: &str) {
    let title = title.trim();
    if title.is_empty() {
        return;
    }
    let normalized = normalize_search_text(title);
    if normalized.chars().count() < 4 {
        return;
    }
    titles.push(title.to_string());
}

fn resource_queries(titles: &[String], episode_number: f64) -> Vec<String> {
    let episode = episode_number.round().max(1.0) as i64;
    let mut queries = Vec::new();
    for raw_title in titles {
        let title = raw_title.trim();
        queries.push(format!("{title} {episode:02}"));
        queries.push(format!("{title} {episode}"));
        queries.push(format!("{title} S01E{episode:02}"));
        if episode > 1 {
            queries.push(format!("{title} batch"));
        }
    }
    dedupe_strings(queries)
}

fn frontend_resource_from_domain(
    candidate: ResourceCandidate,
    episode_number: f64,
) -> EpisodeResourceData {
    let subtitle_group = candidate
        .subtitle_group
        .clone()
        .unwrap_or_else(|| "未知字幕组".to_string());
    let resolution = candidate
        .resolution
        .clone()
        .unwrap_or_else(|| "未知".to_string());
    let score = resource_score(&candidate, episode_number);
    let span = resource_episode_span(&candidate.title, episode_number);
    EpisodeResourceData {
        id: candidate.id,
        provider: candidate.provider,
        title: candidate.title,
        subtitle_group,
        resolution,
        torrent_url: candidate.torrent_url,
        page_url: candidate.page_url.unwrap_or_default(),
        info_hash: candidate.info_hash.unwrap_or_default(),
        size: candidate.size_text.unwrap_or_default(),
        seeders: candidate.seeders,
        leechers: candidate.leechers,
        downloads: candidate.downloads,
        trusted: candidate.trusted,
        remake: candidate.remake,
        batch: candidate.batch,
        episode_start: span.0,
        episode_end: span.1,
        published_at: candidate.published_at.unwrap_or_default(),
        score,
    }
}

fn frontend_download_task_from_domain(task: DownloadTask) -> DownloadTaskData {
    DownloadTaskData {
        id: task.id,
        resource_id: task.resource_id,
        subject_provider: task.subject_provider,
        provider_subject_id: task.provider_subject_id,
        episode_number: task.episode_number,
        title: task.title,
        torrent_url: task.torrent_url,
        info_hash: task.info_hash.unwrap_or_default(),
        qbittorrent_hash: task.qbittorrent_hash.unwrap_or_default(),
        status: task.status,
        progress: task.progress,
        save_path: task.save_path.unwrap_or_default(),
        error: task.error.unwrap_or_default(),
        updated_at: task.updated_at,
    }
}

fn parse_nyaa_rss(xml: &str) -> AppResult<Vec<ResourceCandidate>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut items = Vec::new();
    let mut current = HashMap::<String, String>::new();
    let mut in_item = false;
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = tag_name(event.name().as_ref());
                if name == "item" {
                    in_item = true;
                    current.clear();
                } else if in_item {
                    current_tag = name;
                }
            }
            Ok(Event::End(event)) => {
                let name = tag_name(event.name().as_ref());
                if name == "item" {
                    if let Some(candidate) = resource_from_rss_item(&current) {
                        items.push(candidate);
                    }
                    in_item = false;
                    current_tag.clear();
                } else if in_item {
                    current_tag.clear();
                }
            }
            Ok(Event::Text(text)) => {
                if in_item && !current_tag.is_empty() {
                    let value = text
                        .decode()
                        .map_err(|error| AppError::Api(format!("Nyaa RSS decode failed: {error}")))?
                        .into_owned();
                    current
                        .entry(current_tag.clone())
                        .and_modify(|existing| existing.push_str(&value))
                        .or_insert(value);
                }
            }
            Ok(Event::CData(text)) => {
                if in_item && !current_tag.is_empty() {
                    let value = text
                        .decode()
                        .map_err(|error| {
                            AppError::Api(format!("Nyaa RSS CDATA decode failed: {error}"))
                        })?
                        .into_owned();
                    current.insert(current_tag.clone(), value);
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(AppError::Api(format!("Nyaa RSS parse failed: {error}"))),
            _ => {}
        }
    }
    Ok(items)
}

fn tag_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    name.rsplit(':').next().unwrap_or(&name).to_string()
}

fn resource_from_rss_item(item: &HashMap<String, String>) -> Option<ResourceCandidate> {
    let title = item.get("title")?.trim().to_string();
    let torrent_url = item.get("link")?.trim().to_string();
    let page_url = item.get("guid").cloned();
    let info_hash = non_empty(item.get("infoHash").cloned().unwrap_or_default());
    let group = parse_subtitle_group(&title);
    let resolution = parse_resolution(&title);
    let lowered = title.to_ascii_lowercase();
    let batch = lowered.contains("batch")
        || lowered.contains("complete")
        || lowered.contains("mini")
        || parse_episode_range(&title).is_some();
    Some(ResourceCandidate {
        id: 0,
        subject_provider: String::new(),
        provider_subject_id: String::new(),
        episode_number: None,
        provider: "nyaa".to_string(),
        title,
        subtitle_group: group,
        resolution,
        torrent_url,
        page_url,
        info_hash,
        size_text: item.get("size").cloned(),
        seeders: parse_i64(item.get("seeders")),
        leechers: parse_i64(item.get("leechers")),
        downloads: parse_i64(item.get("downloads")),
        trusted: item
            .get("trusted")
            .is_some_and(|value| value.eq_ignore_ascii_case("yes")),
        remake: item
            .get("remake")
            .is_some_and(|value| value.eq_ignore_ascii_case("yes")),
        batch,
        published_at: item.get("pubDate").cloned(),
    })
}

fn resource_matches_title(title: &str, candidates: &[String]) -> bool {
    let title = normalize_resource_title(title);
    candidates.iter().any(|candidate| {
        let candidate = normalize_resource_title(candidate);
        if candidate.chars().count() < 4 {
            return false;
        }
        title.contains(&candidate) || candidate.contains(&title)
    })
}

fn normalize_resource_title(value: &str) -> String {
    normalize_search_text(value)
        .replace("season", "s")
        .replace("mini-episodes", "mini")
        .replace("miniepisodes", "mini")
}

fn resource_matches_episode(title: &str, episode_number: f64) -> bool {
    let target = episode_number.round().max(1.0) as i64;
    let (start, end) = resource_episode_span(title, episode_number);
    start <= target && target <= end
}

fn resource_episode_span(title: &str, episode_number: f64) -> (i64, i64) {
    let target = episode_number.round().max(1.0) as i64;
    if let Some((start, end)) = parse_episode_range(title) {
        return (start, end);
    }
    if let Some(episode) = parse_episode_number(title) {
        return (episode, episode);
    }
    let lowered = title.to_ascii_lowercase();
    if lowered.contains("batch") || lowered.contains("complete") || lowered.contains("mini") {
        return (1, 999);
    }
    (target, target)
}

fn resource_score(candidate: &ResourceCandidate, episode_number: f64) -> i64 {
    let mut score = candidate.seeders * 4 + candidate.downloads / 50 - candidate.leechers;
    if candidate.trusted {
        score += 500;
    }
    if candidate.remake {
        score -= 150;
    }
    if resource_matches_episode(&candidate.title, episode_number) {
        score += 200;
    }
    if candidate.batch {
        score -= 40;
    }
    if candidate.resolution.as_deref() == Some("1080p") {
        score += 40;
    }
    score
}

fn parse_subtitle_group(title: &str) -> Option<String> {
    let regex = Regex::new(r"^\[([^\]]+)\]").ok()?;
    regex
        .captures(title)
        .and_then(|captures| captures.get(1))
        .map(|group| group.as_str().trim().to_string())
        .filter(|group| !group.is_empty())
}

fn parse_resolution(title: &str) -> Option<String> {
    let regex = Regex::new(r"(?i)(2160p|1440p|1080p|720p|480p)").ok()?;
    regex
        .captures(title)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_ascii_lowercase())
}

fn parse_episode_number(title: &str) -> Option<i64> {
    let patterns = [
        r"(?i)S\d{1,2}E(\d{1,3})",
        r"(?i)\bE(\d{1,3})\b",
        r"(?:^|[\s\-_])(\d{1,3})(?:[\s\]_.-]|$)",
    ];
    for pattern in patterns {
        let regex = Regex::new(pattern).ok()?;
        if let Some(value) = regex
            .captures(title)
            .and_then(|captures| captures.get(1))
            .and_then(|value| value.as_str().parse::<i64>().ok())
        {
            return Some(value);
        }
    }
    None
}

fn parse_episode_range(title: &str) -> Option<(i64, i64)> {
    let patterns = [
        r"[\(\[]\s*(\d{1,3})\s*[-~]\s*(\d{1,3})\s*[\)\]]",
        r"(?i)S\d{1,2}E(\d{1,3})\s*[-~]\s*E?(\d{1,3})",
    ];
    for pattern in patterns {
        let regex = Regex::new(pattern).ok()?;
        if let Some(captures) = regex.captures(title) {
            let start = captures.get(1)?.as_str().parse::<i64>().ok()?;
            let end = captures.get(2)?.as_str().parse::<i64>().ok()?;
            return Some((start.min(end), start.max(end)));
        }
    }
    None
}

fn parse_i64(value: Option<&String>) -> i64 {
    value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or_default()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let key = value.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(value);
        }
    }
    out
}

fn push_string(values: &mut Vec<String>, value: Option<String>) {
    if let Some(value) = value {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            values.push(trimmed.to_string());
        }
    }
}

fn strip_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(character),
            _ => {}
        }
    }
    out.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&#039;", "'")
        .replace("<br>", "\n")
        .trim()
        .to_string()
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn ensure_http_success(status: reqwest::StatusCode, action: &str) -> AppResult<()> {
    if status.is_success() {
        Ok(())
    } else {
        Err(AppError::Api(format!("{action} rejected: {status}")))
    }
}

fn ensure_dandanplay_success(
    success: Option<bool>,
    error_code: Option<i64>,
    error_message: Option<String>,
) -> AppResult<()> {
    if success.unwrap_or(true) {
        Ok(())
    } else {
        Err(AppError::Api(format!(
            "dandanplay error {}: {}",
            error_code.unwrap_or_default(),
            error_message.unwrap_or_else(|| "unknown error".to_string())
        )))
    }
}

fn unix_timestamp_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn dandanplay_signature(app_id: &str, timestamp: i64, path: &str, app_secret: &str) -> String {
    let source = format!("{app_id}{timestamp}{path}{app_secret}");
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    BASE64_STANDARD.encode(hasher.finalize())
}

const DANDANPLAY_BASE_URL: &str = "https://api.dandanplay.net";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListSearchResponse {
    data: AniListSearchData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AniListSearchData {
    page: AniListPage,
}

#[derive(Debug, Deserialize)]
struct AniListPage {
    media: Vec<AniListMedia>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListDetailResponse {
    data: AniListDetailData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AniListDetailData {
    media: Option<AniListMedia>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListMedia {
    id: i64,
    title: AniListTitle,
    #[serde(default)]
    synonyms: Vec<String>,
    description: Option<String>,
    start_date: Option<AniListDate>,
    episodes: Option<i64>,
    cover_image: Option<AniListCover>,
    average_score: Option<i64>,
}

impl AniListMedia {
    fn aliases(&self) -> Vec<String> {
        let mut aliases = self.synonyms.clone();
        push_string(&mut aliases, self.title.romaji.clone());
        push_string(&mut aliases, self.title.english.clone());
        push_string(&mut aliases, self.title.native.clone());
        dedupe_strings(aliases)
    }
}

#[derive(Debug, Deserialize)]
struct AniListTitle {
    romaji: Option<String>,
    english: Option<String>,
    native: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AniListDate {
    year: Option<i32>,
    month: Option<u8>,
    day: Option<u8>,
}

impl AniListDate {
    fn to_iso_date(&self) -> Option<String> {
        Some(format!(
            "{:04}-{:02}-{:02}",
            self.year?,
            self.month.unwrap_or(1),
            self.day.unwrap_or(1)
        ))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AniListCover {
    large: Option<String>,
    extra_large: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DandanSearchAnimeResponse {
    success: Option<bool>,
    error_code: Option<i64>,
    error_message: Option<String>,
    animes: Option<Vec<DandanSearchAnimeDetails>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DandanSearchAnimeDetails {
    anime_id: Option<i64>,
    anime_title: Option<String>,
    image_url: Option<String>,
    start_date: Option<String>,
    episode_count: Option<i64>,
    rating: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DandanBangumiDetailResponse {
    success: Option<bool>,
    error_code: Option<i64>,
    error_message: Option<String>,
    bangumi: Option<DandanBangumiDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DandanBangumiDetail {
    anime_id: Option<i64>,
    anime_title: Option<String>,
    image_url: Option<String>,
    start_date: Option<String>,
    episode_count: Option<i64>,
    introduction: Option<String>,
    rating: Option<f64>,
    episodes: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Default, Deserialize)]
struct QbitTorrentInfo {
    hash: Option<String>,
    progress: f64,
    #[serde(rename = "save_path")]
    save_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nyaa_rss_candidate_fields() {
        let rss = r#"
        <rss xmlns:nyaa="https://nyaa.si/xmlns/nyaa" version="2.0">
          <channel>
            <item>
              <title>[SubsPlease] Example Anime - 09 (1080p) [ABCD].mkv</title>
              <link>https://nyaa.si/download/1.torrent</link>
              <guid>https://nyaa.si/view/1</guid>
              <pubDate>Fri, 20 Mar 2026 15:02:32 -0000</pubDate>
              <nyaa:seeders>845</nyaa:seeders>
              <nyaa:leechers>2</nyaa:leechers>
              <nyaa:downloads>45749</nyaa:downloads>
              <nyaa:infoHash>7f4873487bfb6fc748f4f485c7a311bbaff6286d</nyaa:infoHash>
              <nyaa:size>1.4 GiB</nyaa:size>
              <nyaa:trusted>Yes</nyaa:trusted>
              <nyaa:remake>No</nyaa:remake>
            </item>
          </channel>
        </rss>
        "#;

        let candidates = parse_nyaa_rss(rss).expect("parse rss");
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.subtitle_group.as_deref(), Some("SubsPlease"));
        assert_eq!(candidate.resolution.as_deref(), Some("1080p"));
        assert_eq!(candidate.seeders, 845);
        assert!(candidate.trusted);
        assert!(resource_matches_episode(&candidate.title, 9.0));
    }

    #[test]
    fn matches_batch_episode_ranges() {
        assert!(resource_matches_episode(
            "[SubsPlease] Example Anime (01-10) (1080p) [Batch]",
            9.0
        ));
        assert!(!resource_matches_episode(
            "[SubsPlease] Example Anime (01-08) (1080p) [Batch]",
            9.0
        ));
        assert!(resource_matches_episode(
            "[SubsPlease] Super no Ura de Yani Suu Futari Mini (01-12) (1080p) [Batch]",
            7.0
        ));
        assert!(resource_matches_episode(
            "[ToonsHub] Smoking Behind the Supermarket with You S00E01-E06 1080p",
            6.0
        ));
        assert!(!resource_matches_episode(
            "[ToonsHub] Smoking Behind the Supermarket with You S00E01-E06 1080p",
            7.0
        ));
    }

    #[test]
    fn rejects_nyaa_results_for_different_titles() {
        let titles = resource_title_candidates(
            "Super no Ura de Yani Suu Futari",
            "在超市后门吸烟的二人",
            &["Smoking Behind the Supermarket with You".to_string()],
        );
        let queries = resource_queries(&titles, 7.0);
        assert!(
            !queries
                .iter()
                .any(|query| query == "超市后门 07" || query == "超市后门 batch")
        );
        assert!(resource_matches_title(
            "[SubsPlease] Super no Ura de Yani Suu Futari Mini (01-12) (1080p) [Batch]",
            &titles
        ));
        assert!(!resource_matches_title(
            "[SubsPlease] Completely Different Anime (01-12) (1080p) [Batch]",
            &titles
        ));
    }

    #[test]
    fn extracts_cjk_keyword_windows_without_title_specific_aliases() {
        let variants = search_query_variants("在超市后门邂逅的两个人");
        assert!(variants.contains(&"超市后门".to_string()));
        assert!(!variants.contains(&"スーパーの裏でヤニ吸うふたり".to_string()));
        assert!(!variants.contains(&"Super no Ura de Yani Suu Futari".to_string()));
    }

    #[test]
    fn relevance_prefers_shared_title_fragments() {
        let matching = CatalogSubjectData {
            id: "matching".to_string(),
            provider: "bangumi".to_string(),
            provider_subject_id: "1".to_string(),
            source: "bangumi".to_string(),
            title: "スーパーの裏でヤニ吸うふたり".to_string(),
            title_cn: "在超市后门吸烟的二人".to_string(),
            summary: String::new(),
            air_date: String::new(),
            rating: 0.0,
            rank: 0,
            poster: String::new(),
            hero: String::new(),
            episodes: 0,
            files: 0,
            local: false,
            metadata_ready: true,
            tags: Vec::new(),
            aliases: Vec::new(),
        };
        let unrelated = CatalogSubjectData {
            title: "アニ＊クリ15".to_string(),
            title_cn: "NHK15个动画短片".to_string(),
            provider_subject_id: "2".to_string(),
            id: "unrelated".to_string(),
            ..matching.clone()
        };
        let query = "在超市后门邂逅的两个人";
        assert!(online_subject_score(&matching, query) > online_subject_score(&unrelated, query));
    }

    #[test]
    fn merges_equivalent_subjects_across_providers() {
        let bangumi = test_subject(
            "bangumi",
            "571784",
            "スーパーの裏でヤニ吸うふたり",
            "在超市后门吸烟的二人",
            vec!["Super no Ura de Yani Suu Futari".to_string()],
        );
        let anilist = test_subject(
            "anilist",
            "196187",
            "Super no Ura de Yani Suu Futari",
            "スーパーの裏でヤニ吸うふたり",
            vec!["Smoking Behind the Supermarket with You".to_string()],
        );
        let merged = merge_equivalent_subjects(vec![anilist, bangumi]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].provider, "bangumi");
        assert_eq!(merged[0].provider_subject_id, "571784");
        assert!(
            merged[0]
                .aliases
                .contains(&"Smoking Behind the Supermarket with You".to_string())
        );
    }

    fn test_subject(
        provider: &str,
        provider_subject_id: &str,
        title: &str,
        title_cn: &str,
        aliases: Vec<String>,
    ) -> CatalogSubjectData {
        CatalogSubjectData {
            id: format!("{provider}-{provider_subject_id}"),
            provider: provider.to_string(),
            provider_subject_id: provider_subject_id.to_string(),
            source: provider.to_string(),
            title: title.to_string(),
            title_cn: title_cn.to_string(),
            summary: String::new(),
            air_date: String::new(),
            rating: 0.0,
            rank: 0,
            poster: String::new(),
            hero: String::new(),
            episodes: 0,
            files: 0,
            local: false,
            metadata_ready: true,
            tags: Vec::new(),
            aliases,
        }
    }
}
