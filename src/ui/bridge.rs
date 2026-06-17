use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

use slint::{ModelRc, SharedString, VecModel, Weak};

use crate::MainWindow;
use crate::app::AppContext;
use crate::domain::MediaItem;
use crate::error::{AppError, AppResult};
use crate::task::AppEvent;

#[derive(Default)]
struct UiState {
    media: Vec<MediaItem>,
    selected_index: Option<usize>,
}

#[derive(Clone)]
pub struct BridgeState {
    inner: Arc<Mutex<UiState>>,
}

pub fn bind(window: Weak<MainWindow>, context: AppContext) -> AppResult<BridgeState> {
    let state = Arc::new(Mutex::new(UiState {
        media: context.media.list_media()?,
        selected_index: None,
    }));

    if let Some(window) = window.upgrade() {
        let media = state.lock().expect("ui state mutex poisoned").media.clone();
        set_media_items(&window, &media);
        window.set_logs("ready\n".into());
        window.set_scan_progress("idle".into());
        window.set_settings_summary(context.media.settings_summary(media.len()).into());
    }

    bind_add_path(window.clone(), context.clone(), state.clone());
    bind_scan(window.clone(), context.clone());
    bind_select_media(window.clone(), context.clone(), state.clone());
    bind_play_selected_media(window.clone(), context.clone(), state.clone());
    bind_save_progress(window.clone(), context.clone(), state.clone());
    bind_clear_progress(window.clone(), context.clone(), state.clone());
    bind_load_danmaku(window, context, state.clone());

    Ok(BridgeState { inner: state })
}

pub fn start_event_pump(
    window: Weak<MainWindow>,
    context: AppContext,
    state: BridgeState,
) -> AppResult<()> {
    let receiver = context
        .event_receiver
        .lock()
        .expect("event receiver mutex poisoned")
        .take()
        .ok_or_else(|| AppError::Config("event pump already started".to_string()))?;

    thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            let window = window.clone();
            let state = state.clone();
            let context = context.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = window.upgrade() {
                    apply_event(&window, &context, &state, event);
                }
            });
        }
    });

    Ok(())
}

fn bind_add_path(window: Weak<MainWindow>, context: AppContext, state: Arc<Mutex<UiState>>) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_add_media_path(move |path| {
            let path = path.trim().to_string();
            if path.is_empty() {
                append_log(&weak, "media path is empty");
                return;
            }

            match context.media.add_library_path(path.into()) {
                Ok(paths) => {
                    append_log(
                        &weak,
                        &format!("added media path; total paths: {}", paths.len()),
                    );
                    if let Some(window) = weak.upgrade() {
                        let media_count =
                            state.lock().expect("ui state mutex poisoned").media.len();
                        window.set_settings_summary(
                            context.media.settings_summary(media_count).into(),
                        );
                    }
                }
                Err(error) => append_log(&weak, &format!("add path failed: {error}")),
            }
        });
}

fn bind_play_selected_media(
    window: Weak<MainWindow>,
    context: AppContext,
    state: Arc<Mutex<UiState>>,
) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_play_selected_media(move || {
            let Some(media) = selected_media(&state) else {
                append_log(&weak, "select a media item before playing");
                return;
            };

            if let Err(error) = context.media.open_media(&media) {
                append_log(&weak, &format!("play failed: {error}"));
            }
        });
}

fn bind_scan(window: Weak<MainWindow>, context: AppContext) {
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_scan_library(move || {
            context.media.start_scan();
        });
}

fn bind_select_media(window: Weak<MainWindow>, context: AppContext, state: Arc<Mutex<UiState>>) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_select_media(move |index| {
            let selected = {
                let mut state = state.lock().expect("ui state mutex poisoned");
                let index = index as usize;
                state.selected_index = Some(index);
                state.media.get(index).cloned()
            };

            match selected {
                Some(media) => {
                    if let Some(window) = weak.upgrade() {
                        window.set_selected_info(media_info(&media).into());
                    }
                    match context.watch_history.load(media.id) {
                        Ok(Some(progress)) => append_log(
                            &weak,
                            &format!(
                                "loaded progress: media #{} {} / {} ms",
                                progress.media_id, progress.position_ms, progress.duration_ms
                            ),
                        ),
                        Ok(None) => append_log(&weak, "no watch progress for selected media"),
                        Err(error) => append_log(&weak, &format!("load progress failed: {error}")),
                    }
                }
                None => append_log(&weak, "invalid media selection"),
            }
        });
}

fn bind_save_progress(window: Weak<MainWindow>, context: AppContext, state: Arc<Mutex<UiState>>) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_save_progress(move || {
            let media_id = selected_media_id(&state);
            match media_id.and_then(|id| context.watch_history.save_test_progress(id).ok()) {
                Some(progress) => append_log(
                    &weak,
                    &format!(
                        "test progress saved at {} ms, updated_at={}",
                        progress.position_ms, progress.updated_at
                    ),
                ),
                None => append_log(&weak, "select a media item before saving progress"),
            }
        });
}

fn bind_clear_progress(window: Weak<MainWindow>, context: AppContext, state: Arc<Mutex<UiState>>) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_clear_progress(move || {
            let Some(media_id) = selected_media_id(&state) else {
                append_log(&weak, "select a media item before clearing progress");
                return;
            };

            match context.watch_history.clear(media_id) {
                Ok(()) => append_log(&weak, "progress cleared"),
                Err(error) => append_log(&weak, &format!("clear progress failed: {error}")),
            }
        });
}

fn bind_load_danmaku(window: Weak<MainWindow>, context: AppContext, state: Arc<Mutex<UiState>>) {
    let weak = window.clone();
    window
        .upgrade()
        .expect("window dropped before callback binding")
        .on_load_danmaku(move || {
            let Some(media) = selected_media(&state) else {
                append_log(&weak, "select a media item before loading danmaku");
                return;
            };

            let danmaku = context.danmaku.clone();
            thread::spawn(move || {
                danmaku.load_for_media(&media);
            });
        });
}

fn apply_event(window: &MainWindow, context: &AppContext, state: &BridgeState, event: AppEvent) {
    match event {
        AppEvent::Log(message) => append_log_to_window(window, &message),
        AppEvent::ScanStarted => {
            window.set_scan_progress("scan started".into());
            append_log_to_window(window, "scan started");
        }
        AppEvent::ScanProgress { scanned, indexed } => {
            window.set_scan_progress(format!("scanned {scanned}, indexed {indexed}").into());
        }
        AppEvent::ScanFinished { summary, media } => {
            replace_media_state(state, media.clone());
            set_media_items(window, &media);
            window.set_settings_summary(context.media.settings_summary(media.len()).into());
            window.set_scan_progress(
                format!(
                    "done: scanned={}, added={}, modified={}, restored={}, deleted={}",
                    summary.scanned_files,
                    summary.added,
                    summary.modified,
                    summary.restored,
                    summary.deleted
                )
                .into(),
            );
            append_log_to_window(window, "scan finished");
        }
        AppEvent::ScanFailed(error) => {
            window.set_scan_progress("scan failed".into());
            append_log_to_window(window, &format!("scan failed: {error}"));
        }
        AppEvent::DanmakuMatched(result) => append_log_to_window(
            window,
            &format!(
                "danmaku event: {} matched {} comments, title={}, episode={}",
                result.provider,
                result.comment_count,
                result.title,
                result.episode.as_deref().unwrap_or("(none)")
            ),
        ),
    }
}

fn replace_media_state(state: &BridgeState, media: Vec<MediaItem>) {
    let mut state = state.inner.lock().expect("ui state mutex poisoned");
    state.media = media;
    state.selected_index = None;
}

fn set_media_items(window: &MainWindow, media: &[MediaItem]) {
    let labels = media
        .iter()
        .map(|item| SharedString::from(item.display_label()))
        .collect::<Vec<_>>();
    window.set_media_items(ModelRc::from(Rc::new(VecModel::from(labels))));
}

fn selected_media_id(state: &Arc<Mutex<UiState>>) -> Option<i64> {
    selected_media(state).map(|media| media.id)
}

fn selected_media(state: &Arc<Mutex<UiState>>) -> Option<MediaItem> {
    let state = state.lock().expect("ui state mutex poisoned");
    state
        .selected_index
        .and_then(|index| state.media.get(index).cloned())
}

fn append_log(window: &Weak<MainWindow>, message: &str) {
    if let Some(window) = window.upgrade() {
        append_log_to_window(&window, message);
    }
}

fn append_log_to_window(window: &MainWindow, message: &str) {
    const MAX_LOG_BYTES: usize = 24 * 1024;

    let mut logs = window.get_logs().to_string();
    logs.push_str(message);
    logs.push('\n');
    if logs.len() > MAX_LOG_BYTES {
        let mut keep_from = logs.len() - MAX_LOG_BYTES;
        while !logs.is_char_boundary(keep_from) {
            keep_from += 1;
        }
        let keep_from = logs[keep_from..]
            .find('\n')
            .map(|offset| keep_from + offset + 1)
            .unwrap_or(keep_from);
        logs.replace_range(..keep_from, "");
    }
    window.set_logs(logs.into());
}

fn media_info(media: &MediaItem) -> String {
    format!(
        "id: {}\nfile: {}\npath: {}\nsize: {} bytes\nmodified_at: {}\nhash: {}",
        media.id,
        media.file_name,
        media.path.display(),
        media.file_size,
        media.modified_at,
        media.file_hash.as_deref().unwrap_or("(not computed)")
    )
}
