use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::domain::{DanmakuMatch, MediaItem};

pub fn keyword_for_media(media: &MediaItem, danmaku: Option<&DanmakuMatch>) -> String {
    if let Some(danmaku) = danmaku {
        let title = danmaku
            .title
            .split(" - ")
            .next()
            .unwrap_or(&danmaku.title)
            .trim();
        if !title.is_empty() {
            return title.to_string();
        }
    }

    clean_file_title(&media.file_name)
}

pub fn is_supplemental_video(file_name: &str) -> bool {
    let normalized = file_name
        .to_ascii_lowercase()
        .replace(['_', '.', '-', '[', ']', '(', ')'], " ");
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let joined = tokens.join(" ");

    tokens.iter().any(|token| {
        matches!(
            *token,
            "menu" | "pv" | "cm" | "op" | "ed" | "ncop" | "nced" | "sp" | "sps" | "bonus"
        ) || ["menu", "pv", "ncop", "nced", "sp"]
            .iter()
            .any(|marker| is_numbered_marker(token, marker))
            || token.starts_with("special")
    }) || joined.contains("web preview")
        || joined.contains("creditless")
        || joined.contains("clean opening")
        || joined.contains("clean ending")
        || joined.contains("textless")
        || joined.contains("trailer")
        || joined.contains("preview")
        || joined.contains("drama")
        || joined.contains("reminder")
}

fn is_numbered_marker(token: &str, marker: &str) -> bool {
    token
        .strip_prefix(marker)
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

pub fn episode_number_from_file_name(file_name: &str) -> Option<f64> {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name);

    episode_patterns().iter().find_map(|pattern| {
        pattern
            .captures(stem)
            .and_then(|captures| captures.name("episode"))
            .and_then(|value| value.as_str().parse::<f64>().ok())
            .filter(|value| *value > 0.0 && *value <= 9999.0)
    })
}

pub fn series_key_from_file_name(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name);
    let (series_part, season) = season_episode_pattern()
        .captures(stem)
        .and_then(|captures| {
            let whole = captures.get(0)?;
            let season = captures
                .name("season")
                .map(|value| value.as_str().trim_start_matches('0'))
                .filter(|value| !value.is_empty())
                .unwrap_or("0");
            Some((&stem[..whole.start()], Some(season)))
        })
        .or_else(|| {
            episode_patterns().iter().skip(1).find_map(|pattern| {
                pattern
                    .find(stem)
                    .map(|matched| (&stem[..matched.start()], None))
            })
        })
        .unwrap_or((stem, None));

    let mut key = series_part
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if let Some(season) = season {
        key.push_str("season");
        key.push_str(season);
    }
    if key.is_empty() {
        stem.to_lowercase()
    } else {
        key
    }
}

fn season_episode_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?i)s(?P<season>\d{1,2})[ ._-]*e(?P<episode>\d{1,4})")
            .expect("valid season/episode regex")
    })
}

fn episode_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"(?i)s(?P<season>\d{1,2})[ ._-]*e(?P<episode>\d{1,4})",
            r"(?i)(?:^|[ ._\-\[(])(?:episode|ep|e)[ ._\-]*(?P<episode>\d{1,4})",
            r"第\s*(?P<episode>\d{1,4}(?:\.\d+)?)\s*[话話集]",
            r"\[(?P<episode>\d{1,3}(?:\.\d+)?)\]",
            r"(?:^|\s)-\s*(?P<episode>\d{1,3}(?:\.\d+)?)(?:v\d+)?(?:\s|$)",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("valid episode regex"))
        .collect()
    })
}

fn clean_file_title(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name);
    let mut out = String::new();
    let mut bracket_depth = 0;
    for ch in stem.chars() {
        match ch {
            '[' | '【' | '(' | '（' => bracket_depth += 1,
            ']' | '】' | ')' | '）' => bracket_depth = (bracket_depth - 1).max(0),
            '_' | '.' if bracket_depth == 0 => out.push(' '),
            _ if bracket_depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_episode_before_release_numbers() {
        let file = "サムライチャンプルー.Samurai.Champloo.S01E15.2004.BluRay.1080p.HEVC.10bit.mkv";
        assert_eq!(episode_number_from_file_name(file), Some(15.0));
        assert!(series_key_from_file_name(file).ends_with("season1"));
    }

    #[test]
    fn extracts_common_episode_notations() {
        assert_eq!(
            episode_number_from_file_name("Anime - 03 [1080p].mkv"),
            Some(3.0)
        );
        assert_eq!(
            episode_number_from_file_name("Anime 第12話.mkv"),
            Some(12.0)
        );
        assert_eq!(
            episode_number_from_file_name("Anime [07] [10bit].mkv"),
            Some(7.0)
        );
    }

    #[test]
    fn keeps_seasons_in_separate_series_keys() {
        assert_ne!(
            series_key_from_file_name("Anime.S01E01.mkv"),
            series_key_from_file_name("Anime.S02E01.mkv")
        );
    }

    #[test]
    fn normal_titles_starting_with_sp_are_not_supplemental() {
        assert!(!is_supplemental_video("SPY x FAMILY S01E01.mkv"));
        assert!(!is_supplemental_video("Space Dandy - 01.mkv"));
        assert!(is_supplemental_video("Anime SP01.mkv"));
        assert!(is_supplemental_video("Anime Special 01.mkv"));
    }
}
