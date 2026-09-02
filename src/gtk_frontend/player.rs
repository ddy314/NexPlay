use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;

use crate::backend_api::{
    FrontendEpisode, FrontendSubject, MediaSourceRequest, MediaSourceResponse,
    PlaybackProgressRequest, PlaybackSessionFinishRequest, PlaybackSessionHeartbeatRequest,
    PlaybackSessionStartRequest, PlaybackSessionStartResponse, finish_playback_session,
    heartbeat_playback_session, media_source, report_playback_progress, start_playback_session,
};

use super::{UiState, clear_box, label, request_snapshot, status, subject_title};

const TICK_MILLIS: i64 = 1_000;
const HEARTBEAT_TICKS: u64 = 15;

pub(super) fn open_player(state: &Rc<UiState>, subject: FrontendSubject, episode: FrontendEpisode) {
    let Some(media_id) = episode.media_id else {
        super::show_error(state, "这一集还没有本地媒体文件".to_string());
        return;
    };

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.append(&status(
        "正在准备播放器",
        "正在验证本地文件并读取上次观看位置…",
        "content-loading-symbolic",
    ));
    let tag = format!("player-{}", state.next_page_id.get());
    state
        .next_page_id
        .set(state.next_page_id.get().saturating_add(1));
    let title = episode_title(&episode);
    let page = adw::NavigationPage::with_tag(&content, &title, &tag);
    state.navigation.push(&page);

    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| media_source(context, MediaSourceRequest { media_id }),
        move |result: Result<MediaSourceResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            clear_box(&content);
            match result {
                Ok(source) => render_player(&state, &content, &page, subject, episode, source),
                Err(error) => {
                    content.append(&status("无法播放本地文件", &error, "dialog-error-symbolic"))
                }
            }
        },
    );
}

fn render_player(
    state: &Rc<UiState>,
    container: &gtk::Box,
    page: &adw::NavigationPage,
    subject: FrontendSubject,
    episode: FrontendEpisode,
    source: MediaSourceResponse,
) {
    let file = gio::File::for_uri(&source.source_url);
    let stream = gtk::MediaFile::for_file(&file);
    let video = gtk::Video::builder()
        .media_stream(&stream)
        .autoplay(true)
        .hexpand(true)
        .vexpand(true)
        .build();

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        &episode_title(&episode),
        &subject_title(&subject),
    )));
    toolbar.add_top_bar(&header);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(18);
    body.set_margin_end(18);
    body.set_margin_bottom(18);
    body.append(&video);
    let details = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let file_label = label(&source.file_name, "dim-label");
    file_label.set_hexpand(true);
    let position_label = label("准备播放…", "numeric");
    details.append(&file_label);
    details.append(&position_label);
    body.append(&details);
    toolbar.set_content(Some(&body));
    container.append(&toolbar);

    let active = Rc::new(Cell::new(true));
    let finished = Rc::new(Cell::new(false));
    let session_id = Rc::new(Cell::new(0_i64));
    let session_ready = Rc::new(Cell::new(false));
    let active_ms = Rc::new(Cell::new(0_i64));
    let ticks = Rc::new(Cell::new(0_u64));
    let started = Rc::new(Cell::new(false));

    stream.connect_prepared_notify({
        let state = state.clone();
        let stream = stream.clone();
        let started = started.clone();
        let session_id = session_id.clone();
        let session_ready = session_ready.clone();
        let active = active.clone();
        let finished = finished.clone();
        let active_ms = active_ms.clone();
        let source = source.clone();
        let episode = episode.clone();
        let subject = subject.clone();
        move |_| {
            if !stream.is_prepared() || started.replace(true) {
                return;
            }
            let duration = micros_to_seconds(stream.duration());
            let initial = source
                .playback_position
                .filter(|position| *position > 0.0 && *position < duration - 5.0)
                .unwrap_or(0.0);
            if initial > 0.0 && stream.is_seekable() {
                stream.seek(seconds_to_micros(initial));
            }
            let weak = Rc::downgrade(&state);
            let session_id_for_callback = session_id.clone();
            let subject_id = subject.subject_id;
            let episode_id = episode.bgm_episode_id.unwrap_or_default();
            let media_id = source.media_id;
            let session_ready_for_callback = session_ready.clone();
            let active_for_callback = active.clone();
            let stream_for_callback = stream.clone();
            let finished_for_callback = finished.clone();
            let active_ms_for_callback = active_ms.clone();
            let subject_for_callback = subject.clone();
            let episode_for_callback = episode.clone();
            state.runtime.submit(
                move |context| {
                    start_playback_session(
                        context,
                        PlaybackSessionStartRequest {
                            subject_id,
                            episode_id,
                            media_id: Some(media_id),
                            position: initial,
                            duration,
                        },
                    )
                },
                move |result: Result<PlaybackSessionStartResponse, String>| {
                    let Some(state) = weak.upgrade() else { return };
                    let response_session_id = result
                        .map(|response| response.session_id)
                        .unwrap_or_default();
                    session_id_for_callback.set(response_session_id);
                    session_ready_for_callback.set(true);
                    if !active_for_callback.get() || stream_for_callback.is_ended() {
                        finish_once(
                            &state,
                            &finished_for_callback,
                            response_session_id,
                            true,
                            active_ms_for_callback.get(),
                            &subject_for_callback,
                            &episode_for_callback,
                            micros_to_seconds(stream_for_callback.timestamp()),
                            micros_to_seconds(stream_for_callback.duration()),
                            stream_for_callback.is_ended(),
                        );
                    }
                },
            );
        }
    });

    stream.connect_error_notify({
        let state = state.clone();
        move |stream| {
            if let Some(error) = stream.error() {
                super::show_error(&state, format!("播放器错误：{error}"));
            }
        }
    });

    let weak_state = Rc::downgrade(state);
    let weak_stream = stream.downgrade();
    glib::timeout_add_local(Duration::from_millis(TICK_MILLIS as u64), {
        let active = active.clone();
        let finished = finished.clone();
        let session_id = session_id.clone();
        let session_ready = session_ready.clone();
        let active_ms = active_ms.clone();
        let ticks = ticks.clone();
        let position_label = position_label.clone();
        let subject = subject.clone();
        let episode = episode.clone();
        move || {
            let (Some(state), Some(stream)) = (weak_state.upgrade(), weak_stream.upgrade()) else {
                return glib::ControlFlow::Break;
            };
            if !active.get() {
                return glib::ControlFlow::Break;
            }
            let position = micros_to_seconds(stream.timestamp());
            let duration = micros_to_seconds(stream.duration());
            position_label.set_text(&format!(
                "{} / {}",
                format_duration(position),
                format_duration(duration)
            ));
            if stream.is_playing() {
                active_ms.set(active_ms.get().saturating_add(TICK_MILLIS));
            }
            ticks.set(ticks.get().saturating_add(1));

            if stream.is_ended() {
                finish_once(
                    &state,
                    &finished,
                    session_id.get(),
                    session_ready.get(),
                    active_ms.get(),
                    &subject,
                    &episode,
                    position,
                    duration,
                    true,
                );
                return glib::ControlFlow::Break;
            }
            if ticks.get().is_multiple_of(HEARTBEAT_TICKS) {
                persist_progress(
                    &state,
                    session_id.get(),
                    active_ms.get(),
                    &subject,
                    &episode,
                    position,
                    duration,
                );
            }
            glib::ControlFlow::Continue
        }
    });

    let pop_handler = Rc::new(RefCell::new(None));
    let handler = state.navigation.connect_popped({
        let state = state.clone();
        let page = page.clone();
        let stream = stream.clone();
        let active = active.clone();
        let finished = finished.clone();
        let session_id = session_id.clone();
        let session_ready = session_ready.clone();
        let active_ms = active_ms.clone();
        let pop_handler = pop_handler.clone();
        move |navigation, popped| {
            if popped != &page || !active.replace(false) {
                return;
            }
            stream.pause();
            let position = micros_to_seconds(stream.timestamp());
            let duration = micros_to_seconds(stream.duration());
            finish_once(
                &state,
                &finished,
                session_id.get(),
                session_ready.get(),
                active_ms.get(),
                &subject,
                &episode,
                position,
                duration,
                stream.is_ended(),
            );
            if let Some(handler) = pop_handler.borrow_mut().take() {
                navigation.disconnect(handler);
            }
        }
    });
    pop_handler.replace(Some(handler));
}

fn persist_progress(
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
fn finish_once(
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

fn episode_title(episode: &FrontendEpisode) -> String {
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

fn micros_to_seconds(value: i64) -> f64 {
    value.max(0) as f64 / 1_000_000.0
}

fn seconds_to_micros(value: f64) -> i64 {
    (value.max(0.0) * 1_000_000.0).round() as i64
}

fn format_duration(value: f64) -> String {
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
