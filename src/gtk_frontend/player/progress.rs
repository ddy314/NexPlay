use std::cell::Cell;
use std::rc::Rc;

use crate::backend_api::{
    FrontendEpisode, FrontendSubject, PlaybackProgressRequest, PlaybackSessionFinishRequest,
    PlaybackSessionHeartbeatRequest, finish_playback_session, heartbeat_playback_session,
    report_playback_progress,
};

use super::super::{events::request_snapshot, state::UiState};

pub(super) fn persist_progress(
    state: &Rc<UiState>,
    session_id: i64,
    active_ms: i64,
    subject: &FrontendSubject,
    episode: &FrontendEpisode,
    position: f64,
    duration: f64,
) {
    if duration <= 0.0 {
        return;
    }
    let progress = PlaybackProgressRequest {
        subject_id: subject.subject_id,
        episode_id: episode.bgm_episode_id.unwrap_or_default(),
        media_id: episode.media_id,
        position,
        duration,
    };
    state.runtime.submit(
        move |context| report_playback_progress(context, progress),
        |_| {},
    );
    if session_id > 0 {
        state.runtime.submit(
            move |context| {
                heartbeat_playback_session(
                    context,
                    PlaybackSessionHeartbeatRequest {
                        session_id,
                        position,
                        duration,
                        active_ms,
                    },
                )
            },
            |_| {},
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_once(
    state: &Rc<UiState>,
    finished: &Cell<bool>,
    session_id: i64,
    session_ready: bool,
    active_ms: i64,
    subject: &FrontendSubject,
    episode: &FrontendEpisode,
    position: f64,
    duration: f64,
    completed: bool,
) {
    if finished.get() || duration <= 0.0 {
        return;
    }
    persist_progress(
        state, session_id, active_ms, subject, episode, position, duration,
    );
    if !session_ready {
        request_snapshot(state);
        return;
    }
    finished.set(true);
    if session_id > 0 {
        state.runtime.submit(
            move |context| {
                finish_playback_session(
                    context,
                    PlaybackSessionFinishRequest {
                        session_id,
                        position,
                        duration,
                        active_ms,
                        completed,
                        seek_count: 0,
                    },
                )
            },
            |_| {},
        );
    }
    request_snapshot(state);
}

pub(super) fn episode_title(episode: &FrontendEpisode) -> String {
    let title = if episode.title_cn.trim().is_empty() {
        episode.title.trim()
    } else {
        episode.title_cn.trim()
    };
    if title.is_empty() {
        format!("第 {} 集", episode.episode)
    } else {
        format!("第 {} 集 · {title}", episode.episode)
    }
}

pub(super) fn micros_to_seconds(value: i64) -> f64 {
    value.max(0) as f64 / 1_000_000.0
}

pub(super) fn seconds_to_micros(value: f64) -> i64 {
    (value.max(0.0) * 1_000_000.0).round() as i64
}

pub(super) fn format_duration(value: f64) -> String {
    let seconds = value.max(0.0).round() as u64;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}
