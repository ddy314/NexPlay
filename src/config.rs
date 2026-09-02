use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, io_error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub media_libraries: Vec<PathBuf>,
    #[serde(default)]
    pub dandanplay: DandanplayConfig,
    #[serde(default)]
    pub bangumi: BangumiConfig,
    #[serde(default)]
    pub nyaa: NyaaConfig,
    #[serde(default)]
    pub qbittorrent: QbittorrentConfig,
    #[serde(default)]
    pub experience: ExperienceConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperienceConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default = "default_true")]
    pub analytics_enabled: bool,
    #[serde(default = "default_daily_minutes_goal")]
    pub daily_minutes_goal: u64,
    #[serde(default = "default_weekly_episodes_goal")]
    pub weekly_episodes_goal: u64,
    #[serde(default = "default_weekly_active_days_goal")]
    pub weekly_active_days_goal: u64,
}

impl Default for ExperienceConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            reduced_motion: false,
            analytics_enabled: true,
            daily_minutes_goal: default_daily_minutes_goal(),
            weekly_episodes_goal: default_weekly_episodes_goal(),
            weekly_active_days_goal: default_weekly_active_days_goal(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/nexplay.sqlite3"),
        }
    }
}

impl Default for DandanplayConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            api_key: String::new(),
        }
    }
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_true() -> bool {
    true
}

fn default_daily_minutes_goal() -> u64 {
    45
}

fn default_weekly_episodes_goal() -> u64 {
    5
}

fn default_weekly_active_days_goal() -> u64 {
    4
}

fn default_bangumi_base_url() -> String {
    "https://api.bgm.tv".to_string()
}

fn default_bangumi_user_agent() -> String {
    format!("NexPlay/{}", env!("CARGO_PKG_VERSION"))
}

fn default_bangumi_timeout() -> u64 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DandanplayConfig {
    pub app_id: String,
    pub app_secret: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BangumiConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_bangumi_base_url")]
    pub base_url: String,
    #[serde(default = "default_bangumi_oauth_base_url")]
    pub oauth_base_url: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default = "default_bangumi_redirect_uri")]
    pub redirect_uri: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default = "default_bangumi_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_bangumi_timeout")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_true")]
    pub auto_match: bool,
    #[serde(default = "default_true")]
    pub cache_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NyaaConfig {
    pub enabled: bool,
    pub base_url: String,
    pub category: String,
}

impl Default for NyaaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "https://nyaa.si".to_string(),
            category: "0_0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QbittorrentConfig {
    pub enabled: bool,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub save_path: String,
    pub category: String,
    pub tags: String,
}

impl Default for QbittorrentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://127.0.0.1:8080".to_string(),
            username: "admin".to_string(),
            password: String::new(),
            save_path: String::new(),
            category: "NexPlay".to_string(),
            tags: "nexplay".to_string(),
        }
    }
}

impl Default for BangumiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "https://api.bgm.tv".to_string(),
            oauth_base_url: default_bangumi_oauth_base_url(),
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: default_bangumi_redirect_uri(),
            access_token: String::new(),
            user_agent: format!("NexPlay/{}", env!("CARGO_PKG_VERSION")),
            request_timeout_secs: 20,
            auto_match: true,
            cache_images: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

fn default_bangumi_oauth_base_url() -> String {
    "https://bgm.tv".to_string()
}

fn default_bangumi_redirect_uri() -> String {
    "http://127.0.0.1:17654/bangumi/callback".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: PathBuf::from("data/nexplay.sqlite3"),
            },
            media_libraries: Vec::new(),
            dandanplay: DandanplayConfig {
                app_id: String::new(),
                app_secret: String::new(),
                api_key: String::new(),
            },
            bangumi: BangumiConfig::default(),
            nyaa: NyaaConfig::default(),
            qbittorrent: QbittorrentConfig::default(),
            experience: ExperienceConfig::default(),
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }
}

#[derive(Debug)]
pub struct ConfigStore {
    path: PathBuf,
    config: Mutex<AppConfig>,
}

impl ConfigStore {
    pub fn load_or_create(path: impl Into<PathBuf>) -> AppResult<Self> {
        Self::load_or_create_with_default(path, AppConfig::default())
    }

    pub fn load_or_create_with_default(
        path: impl Into<PathBuf>,
        default_config: AppConfig,
    ) -> AppResult<Self> {
        let path = path.into();
        if !path.exists() {
            let mut default_config = default_config;
            resolve_relative_paths(&path, &mut default_config);
            write_config(&path, &default_config)?;
            return Ok(Self {
                path,
                config: Mutex::new(default_config),
            });
        }

        let raw = fs::read_to_string(&path).map_err(|err| io_error(&path, err))?;
        let mut config: AppConfig = toml::from_str(&raw).map_err(|err| {
            AppError::Config(format!("failed to parse {}: {err}", path.display()))
        })?;
        resolve_relative_paths(&path, &mut config);

        Ok(Self {
            path,
            config: Mutex::new(config),
        })
    }

    pub fn snapshot(&self) -> AppConfig {
        self.config.lock().expect("config mutex poisoned").clone()
    }

    pub fn add_media_library(&self, path: PathBuf) -> AppResult<Vec<PathBuf>> {
        if !path.is_dir() {
            return Err(AppError::InvalidMediaDirectory(path));
        }

        let canonical = path
            .canonicalize()
            .map_err(|err| io_error(path.clone(), err))?;

        let mut config = self.config.lock().expect("config mutex poisoned");
        if !config.media_libraries.iter().any(|item| item == &canonical) {
            config.media_libraries.push(canonical);
            write_config(&self.path, &config)?;
        }

        Ok(config.media_libraries.clone())
    }

    pub fn replace(&self, mut next: AppConfig) -> AppResult<AppConfig> {
        let mut canonical_libraries = Vec::new();
        for path in next.media_libraries {
            if path.as_os_str().is_empty() {
                continue;
            }
            if !path.is_dir() {
                return Err(AppError::InvalidMediaDirectory(path));
            }
            let canonical = path.canonicalize().map_err(|err| io_error(path, err))?;
            if !canonical_libraries.iter().any(|item| item == &canonical) {
                canonical_libraries.push(canonical);
            }
        }
        next.media_libraries = canonical_libraries;

        let mut config = self.config.lock().expect("config mutex poisoned");
        *config = next;
        write_config(&self.path, &config)?;
        Ok(config.clone())
    }
}

fn resolve_relative_paths(config_path: &Path, config: &mut AppConfig) {
    let Some(base) = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    else {
        return;
    };
    if config.database.path.is_relative() {
        config.database.path = base.join(&config.database.path);
    }
    for library in &mut config.media_libraries {
        if library.is_relative() {
            *library = base.join(&*library);
        }
    }
}

fn write_config(path: &Path, config: &AppConfig) -> AppResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| io_error(parent, err))?;
    }

    let raw = toml::to_string_pretty(config)
        .map_err(|err| AppError::Config(format!("failed to serialize config: {err}")))?;
    fs::write(path, raw).map_err(|err| io_error(path, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_config_uses_safe_defaults() {
        let config: AppConfig = toml::from_str("[bangumi]\nenabled = false\n").unwrap();
        assert!(!config.bangumi.enabled);
        assert_eq!(config.bangumi.base_url, "https://api.bgm.tv");
        assert_eq!(config.bangumi.request_timeout_secs, 20);
        assert_eq!(config.experience.theme, "system");
        assert!(config.nyaa.enabled);
    }

    #[test]
    fn relative_paths_follow_an_explicit_config_file() {
        let mut config = AppConfig::default();
        config.media_libraries.push(PathBuf::from("media"));
        resolve_relative_paths(Path::new("/tmp/nexplay/config.toml"), &mut config);
        assert_eq!(
            config.database.path,
            PathBuf::from("/tmp/nexplay/data/nexplay.sqlite3")
        );
        assert_eq!(
            config.media_libraries,
            vec![PathBuf::from("/tmp/nexplay/media")]
        );
    }
}
