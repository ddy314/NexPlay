use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, player, state::UiState};
use super::episodes;

const DETAIL_DYNAMIC_REFRESH_TTL: Duration = Duration::from_secs(6 * 60 * 60);

pub(crate) fn open_subject(state: &Rc<UiState>, subject: FrontendSubject) {
    let detail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    detail.set_vexpand(true);
    detail.set_hexpand(true);
    let tag = format!("subject-{}", state.next_page_id.get());
    state
        .next_page_id
        .set(state.next_page_id.get().saturating_add(1));
    let detail_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let current_subject = Rc::new(RefCell::new(subject.clone()));
    if subject.local && subject.subject_id > 0 {
        let refresh = icon_button("view-refresh-symbolic", "刷新 Bangumi 评分");
        let state_for_refresh = state.clone();
        let current_for_refresh = current_subject.clone();
        let detail_for_refresh = detail.clone();
        refresh.connect_clicked(move |_| {
            episodes::refresh_detail(
                &state_for_refresh,
                current_for_refresh.borrow().clone(),
                current_for_refresh.clone(),
                detail_for_refresh.clone(),
            )
        });
        header.pack_end(&refresh);
    }
    if subject.provider == "bangumi" && subject.subject_id > 0 {
        let sync = icon_button("emblem-synchronizing-symbolic", "同步 Bangumi 状态");
        let state_for_sync = state.clone();
        let subject_id = subject.subject_id;
        sync.connect_clicked(move |_| episodes::sync_subject(&state_for_sync, subject_id));
        header.pack_end(&sync);
    }
    detail_view.add_top_bar(&header);
    detail_view.set_content(Some(&detail));
    let page = adw::NavigationPage::with_tag(&detail_view, &subject_title(&subject), &tag);
    state.navigation.push(&page);

    // The detail shell is useful even when the remote source is slow or
    // unavailable.  Render the persisted subject first and let the network
    // work below update this same page when it is actually needed.
    render_detail(state, &detail, subject.clone());

    if subject.provider != "bangumi" || subject.provider_subject_id.trim().is_empty() {
        return;
    }

    if subject_detail_cache_ready(&subject) {
        start_detail_dynamic_refresh(state, subject, current_subject, detail, false, false);
    } else {
        start_detail_hydration(state, subject, current_subject, detail, page);
    }
}

enum DetailDynamicRefreshResult {
    Local(FrontendSubject),
    Online(FrontendSubjectDynamic),
}

pub(crate) fn detail_refresh_key(subject: &FrontendSubject) -> String {
    format!("{}:{}", subject.provider, subject.provider_subject_id)
}

pub(crate) fn start_detail_dynamic_refresh(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
    force: bool,
    show_failure: bool,
) -> bool {
    let key = detail_refresh_key(&subject);
    {
        let mut in_flight = state.detail_dynamic_in_flight.borrow_mut();
        if in_flight.contains(&key) {
            return false;
        }
        if !force
            && state
                .detail_dynamic_refreshes
                .borrow()
                .get(&key)
                .is_some_and(|started| started.elapsed() < DETAIL_DYNAMIC_REFRESH_TTL)
        {
            return false;
        }
        in_flight.insert(key.clone());
        state
            .detail_dynamic_refreshes
            .borrow_mut()
            .insert(key.clone(), Instant::now());
    }

    let weak = Rc::downgrade(state);
    let subject_for_request = subject.clone();
    let current_for_result = current_subject.clone();
    let key_for_result = key.clone();
    state.runtime.submit(
        move |context| {
            if subject_for_request.local && subject_for_request.subject_id > 0 {
                crate::backend_api::refresh_subject_metadata(
                    context,
                    RefreshSubjectRequest {
                        subject_id: subject_for_request.subject_id,
                    },
                )
                .map(DetailDynamicRefreshResult::Local)
            } else {
                online_subject_dynamic(
                    context,
                    OnlineSubjectRequest {
                        provider: subject_for_request.provider,
                        provider_subject_id: subject_for_request.provider_subject_id,
                    },
                )
                .map(DetailDynamicRefreshResult::Online)
            }
        },
        move |result: Result<DetailDynamicRefreshResult, String>| {
            let Some(state) = weak.upgrade() else { return };
            state
                .detail_dynamic_in_flight
                .borrow_mut()
                .remove(&key_for_result);
            match result {
                Ok(DetailDynamicRefreshResult::Local(updated)) => {
                    current_for_result.replace(updated.clone());
                    clear_box(&container);
                    render_detail(&state, &container, updated);
                }
                Ok(DetailDynamicRefreshResult::Online(dynamic)) => {
                    let mut updated = current_for_result.borrow().clone();
                    apply_subject_dynamic(&mut updated, dynamic);
                    current_for_result.replace(updated.clone());
                    clear_box(&container);
                    render_detail(&state, &container, updated);
                }
                Err(error) if show_failure => {
                    events::show_error(&state, format!("刷新 Bangumi 评分失败：{error}"));
                }
                Err(_) => {
                    // Cached content remains the source of truth when an
                    // opportunistic refresh cannot reach Bangumi.
                }
            }
        },
    );
    true
}

pub(crate) fn apply_subject_dynamic(
    subject: &mut FrontendSubject,
    dynamic: FrontendSubjectDynamic,
) {
    if subject.provider == dynamic.provider
        && subject.provider_subject_id == dynamic.provider_subject_id
    {
        if let Some(rating) = dynamic.rating {
            subject.rating = rating;
        }
        if let Some(rank) = dynamic.rank {
            subject.rank = rank;
        }
    }
}

pub(crate) fn start_detail_hydration(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
    page: adw::NavigationPage,
) {
    let subject_ref = SubjectRef {
        canonical_key: subject.canonical_key.clone(),
        provider: subject.provider.clone(),
        provider_subject_id: subject.provider_subject_id.clone(),
        media_id: subject.local_files.first().map(|file| file.media_id),
    };
    let weak = Rc::downgrade(state);
    let page_for_result = page;
    let current_for_result = current_subject;
    let subject_for_request = subject.clone();
    state.runtime.submit(
        move |context| {
            if subject_for_request.local && subject_for_request.subject_id > 0 {
                hydrate_subject(
                    context,
                    RefreshSubjectRequest {
                        subject_id: subject_for_request.subject_id,
                    },
                )
            } else {
                resolve_subject(context, ResolveSubjectRequest { subject_ref })
            }
        },
        move |result: Result<FrontendSubject, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(subject) => {
                    page_for_result.set_title(&subject_title(&subject));
                    current_for_result.replace(subject.clone());
                    clear_box(&container);
                    render_detail(&state, &container, subject);
                }
                Err(error) => {
                    events::show_error(&state, format!("详情补全失败，已保留当前缓存：{error}"));
                }
            }
        },
    );
}

pub(crate) fn render_detail(state: &Rc<UiState>, container: &gtk::Box, subject: FrontendSubject) {
    let scroll_content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    scroll_content.set_margin_top(18);
    scroll_content.set_margin_bottom(28);
    scroll_content.set_margin_start(28);
    scroll_content.set_margin_end(28);
    let overview = adw::WrapBox::builder()
        .child_spacing(28)
        .line_spacing(24)
        .natural_line_length(920)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    overview.set_hexpand(true);
    overview.set_halign(gtk::Align::Fill);
    overview.set_justify_last_line(false);
    overview.append(&detail_cover(state, &subject));
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 8);
    copy.set_hexpand(true);
    copy.set_width_request(360);
    copy.append(&label(subject_title(&subject), "title-1"));
    if !subject.title.trim().is_empty() && subject.title.trim() != subject_title(&subject) {
        copy.append(&label(&subject.title, "title-3"));
    }
    copy.append(&label(&subject_meta(&subject), "dim-label"));
    let status_text = if subject.local {
        "本地可用"
    } else {
        "在线条目，尚无本地媒体"
    };
    copy.append(&label(status_text, "body"));
    let summary = localized_summary(&subject.summary);
    if !summary.is_empty() {
        copy.append(&label("简介", "heading"));
        let summary_preview = label(summary, "body");
        summary_preview.set_lines(3);
        summary_preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
        summary_preview.set_halign(gtk::Align::Start);
        copy.append(&summary_preview);
        let full_summary = gtk::Button::with_label("查看完整简介");
        full_summary.add_css_class("flat");
        full_summary.set_halign(gtk::Align::Start);
        let state_for_summary = state.clone();
        let title_for_summary = subject_title(&subject);
        let summary_text = summary.to_string();
        full_summary.connect_clicked(move |_| {
            show_summary_dialog(&state_for_summary, &title_for_summary, &summary_text)
        });
        copy.append(&full_summary);
    }
    if !subject.tags.is_empty() {
        copy.append(&label("标签", "heading"));
        let tags = adw::WrapBox::builder()
            .child_spacing(6)
            .line_spacing(6)
            .wrap_policy(adw::WrapPolicy::Minimum)
            .build();
        tags.set_halign(gtk::Align::Start);
        for tag in subject.tags.iter().take(10) {
            let tag_label = label(format!("#{tag}"), "dim-label");
            tag_label.set_wrap(false);
            tag_label.add_css_class("nx-tag");
            tags.append(&tag_label);
        }
        let remaining = subject.tags.len().saturating_sub(10);
        if remaining > 0 {
            tags.append(&label(format!("+{remaining}"), "dim-label"));
        }
        copy.append(&tags);
    }
    copy.append(&label(
        &format!(
            "观看进度：{}/{}（{}%）",
            subject.watched_episodes,
            subject.episodes,
            (subject.progress * 100.0).round()
        ),
        "dim-label",
    ));
    let progress = gtk::ProgressBar::new();
    progress.set_fraction(subject.progress.clamp(0.0, 1.0));
    copy.append(&progress);
    let copy_width = adw::Clamp::new();
    copy_width.set_maximum_size(680);
    copy_width.set_tightening_threshold(400);
    copy_width.set_child(Some(&copy));
    overview.append(&copy_width);
    scroll_content.append(&overview);
    let episodes = gtk::Box::new(gtk::Orientation::Vertical, 8);
    episodes.append(&label(
        format!("集数（{}）", subject.episodes_detail.len()),
        "title-2",
    ));
    episodes.append(&label(
        "点击集数即可播放；没有本地文件的集数会打开资源搜索。",
        "dim-label",
    ));
    let episode_list = episodes::build_episode_list(state, &subject);
    episodes.append(&episode_list);
    scroll_content.append(&episodes);
    let content_clamp = adw::Clamp::new();
    content_clamp.set_maximum_size(1200);
    content_clamp.set_tightening_threshold(760);
    content_clamp.set_child(Some(&scroll_content));
    container.append(&scrolled(&content_clamp));
}

pub(crate) fn detail_cover(state: &Rc<UiState>, subject: &FrontendSubject) -> gtk::Widget {
    let poster = state
        .images
        .widget(&subject.poster, &state.runtime, 220, 308);
    let Some(episode) = episodes::preferred_playback_episode(subject) else {
        return poster;
    };

    let cover = gtk::Button::new();
    cover.set_has_frame(false);
    cover.set_size_request(220, 308);
    cover.set_hexpand(false);
    cover.set_vexpand(false);
    cover.set_halign(gtk::Align::Start);
    cover.set_valign(gtk::Align::Start);
    cover.set_tooltip_text(Some("播放当前集"));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&poster));
    let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(48);
    play_icon.set_opacity(0.0);
    play_icon.set_halign(gtk::Align::Center);
    play_icon.set_valign(gtk::Align::Center);
    overlay.add_overlay(&play_icon);
    cover.set_child(Some(&overlay));
    let motion = gtk::EventControllerMotion::new();
    let icon_for_enter = play_icon.clone();
    motion.connect_enter(move |_, _, _| icon_for_enter.set_opacity(1.0));
    let icon_for_leave = play_icon.clone();
    motion.connect_leave(move |_| icon_for_leave.set_opacity(0.0));
    cover.add_controller(motion);
    let state_for_play = state.clone();
    let subject_for_play = subject.clone();
    cover.connect_clicked(move |_| {
        player::open_player(&state_for_play, subject_for_play.clone(), episode.clone())
    });
    cover.upcast()
}

pub(crate) fn show_summary_dialog(state: &Rc<UiState>, title: &str, summary: &str) {
    let dialog = adw::Dialog::builder()
        .title("完整简介")
        .content_width(680)
        .content_height(520)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(title, "完整简介")));
    let close = icon_button("window-close-symbolic", "关闭");
    let dialog_for_close = dialog.clone();
    close.connect_clicked(move |_| {
        dialog_for_close.close();
    });
    header.pack_end(&close);
    toolbar.add_top_bar(&header);
    let text = label(summary, "body");
    text.set_selectable(true);
    text.set_margin_top(24);
    text.set_margin_bottom(28);
    text.set_margin_start(28);
    text.set_margin_end(28);
    toolbar.set_content(Some(&scrolled(&text)));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&state.window));
}

pub(crate) fn localized_summary(summary: &str) -> &str {
    summary
        .split_once("[简介原文]")
        .map(|(localized, _)| localized)
        .unwrap_or(summary)
        .trim()
}
