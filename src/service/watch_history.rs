use std::sync::mpsc;

use crate::domain::{PlaybackInsightRow, WatchProgress};
use crate::error::AppResult;
use crate::repository::Repository;
use crate::task::{self, AppEvent};

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

    pub fn save(
        &self,
        media_id: i64,
        position_ms: i64,
        duration_ms: i64,
    ) -> AppResult<WatchProgress> {
        let now = task::unix_timestamp_ms();
        self.repository
            .save_progress(media_id, position_ms, duration_ms, now)?;
        Ok(WatchProgress {
            media_id,
            position_ms,
            duration_ms,
            updated_at: now,
        })
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

    pub fn start_session(
        &self,
        media_id: Option<i64>,
        subject_id: i64,
        episode_id: i64,
        position_ms: i64,
        duration_ms: i64,
    ) -> AppResult<i64> {
        self.repository.start_playback_session(
            media_id,
            subject_id,
            episode_id,
            position_ms,
            duration_ms,
            task::unix_timestamp_ms(),
        )
    }

    pub fn heartbeat_session(
        &self,
        session_id: i64,
        position_ms: i64,
        duration_ms: i64,
        active_ms: i64,
    ) -> AppResult<()> {
        self.repository.heartbeat_playback_session(
            session_id,
            position_ms,
            duration_ms,
            active_ms,
            task::unix_timestamp_ms(),
        )
    }

    pub fn record_session_event(
        &self,
        session_id: i64,
        kind: &str,
        position_ms: i64,
    ) -> AppResult<()> {
        self.repository.record_playback_event(
            session_id,
            kind,
            position_ms,
            task::unix_timestamp_ms(),
        )
    }

    pub fn finish_session(
        &self,
        session_id: i64,
        position_ms: i64,
        duration_ms: i64,
        active_ms: i64,
        completed: bool,
        seek_count: i64,
    ) -> AppResult<()> {
        self.repository.finish_playback_session(
            session_id,
            position_ms,
            duration_ms,
            active_ms,
            completed,
            seek_count,
            task::unix_timestamp_ms(),
        )
    }

    pub fn insight_rows_since(&self, since: i64) -> AppResult<Vec<PlaybackInsightRow>> {
        self.repository.playback_insight_rows_since(since)
    }

    pub fn local_day_key(&self, timestamp_ms: i64) -> AppResult<i64> {
        self.repository.local_day_key(timestamp_ms)
    }

    pub fn clear_analytics(&self) -> AppResult<()> {
        self.repository.clear_playback_analytics()
    }
}
