use super::prelude::*;
use super::{pages, state::UiState};

pub(crate) fn request_snapshot(state: &Rc<UiState>) {
    if state.snapshot_loading.replace(true) {
        return;
    }
    pages::home::render_home(state);
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| snapshot(context),
        move |result: Result<BackendSnapshot, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.snapshot_loading.set(false);
            match result {
                Ok(snapshot) => {
                    state.snapshot.replace(snapshot);
                    render_all(&state);
                }
                Err(error) => show_error(&state, format!("读取资料库失败：{error}")),
            }
        },
    );
}

pub(crate) fn render_all(state: &Rc<UiState>) {
    pages::home::render_home(state);
    pages::discover::render_discover(state);
    pages::library::render_library(state);
    pages::search::render_search(state);
    pages::downloads::render_downloads(state);
    pages::insights::render_insights(state);
    pages::settings::render_settings(state);
}

#[derive(Default)]
pub(crate) struct EventEffects {
    pub(crate) refresh_snapshot: bool,
    pub(crate) render_library: bool,
    pub(crate) render_home: bool,
    pub(crate) render_downloads: bool,
}

impl EventEffects {
    pub(crate) fn merge(&mut self, other: Self) {
        self.refresh_snapshot |= other.refresh_snapshot;
        self.render_library |= other.render_library;
        self.render_home |= other.render_home;
        self.render_downloads |= other.render_downloads;
    }
}

pub(crate) fn handle_event(state: &Rc<UiState>, event: BackendEvent) -> EventEffects {
    if let Some(message) = event.message.as_deref() {
        let mut logs = state.logs.borrow_mut();
        logs.push(message.to_string());
        if logs.len() > 250 {
            let drain = logs.len() - 250;
            logs.drain(0..drain);
        }
        *state.scan_message.borrow_mut() = message.to_string();
    }
    match event.event_type {
        BackendEventType::ScanStarted => {
            state.scan_loading.set(true);
            state.scan_fraction.set(0.0);
            EventEffects {
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::ScanProgress => {
            if let (Some(scanned), Some(indexed)) = (event.scanned, event.indexed) {
                state.scan_fraction.set(if scanned == 0 {
                    0.0
                } else {
                    (indexed as f64 / scanned as f64).clamp(0.0, 1.0)
                });
            }
            EventEffects {
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::ScanFinished | BackendEventType::ScanFailed => {
            state.scan_loading.set(false);
            EventEffects {
                refresh_snapshot: true,
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::SubjectUpdated | BackendEventType::ImageCached => EventEffects {
            refresh_snapshot: true,
            ..EventEffects::default()
        },
        BackendEventType::BangumiSyncStarted => {
            state.sync_loading.set(true);
            state.sync_fraction.set(0.0);
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::BangumiSyncProgress => {
            if let (Some(processed), Some(total)) = (event.processed, event.total) {
                state.sync_fraction.set(if total == 0 {
                    0.0
                } else {
                    (processed as f64 / total as f64).clamp(0.0, 1.0)
                });
            }
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::BangumiSyncFinished => {
            state.sync_loading.set(false);
            state.sync_fraction.set(1.0);
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                refresh_snapshot: true,
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::DownloadCompleted => {
            state.downloads_data.replace(None);
            state.downloads_requested.set(false);
            EventEffects {
                refresh_snapshot: true,
                render_downloads: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::MetadataFailed | BackendEventType::BangumiSyncFailed => {
            if matches!(event.event_type, BackendEventType::BangumiSyncFailed) {
                state.sync_loading.set(false);
            }
            if let Some(message) = event.message {
                show_error(state, message);
            }
            EventEffects {
                refresh_snapshot: true,
                render_home: true,
                ..EventEffects::default()
            }
        }
        _ => EventEffects::default(),
    }
}

pub(crate) fn show_error(state: &Rc<UiState>, message: String) {
    state.toast.add_toast(adw::Toast::new(&message));
}

pub(crate) fn show_success(state: &Rc<UiState>, message: impl Into<String>) {
    state.toast.add_toast(adw::Toast::new(&message.into()));
}
