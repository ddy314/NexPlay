use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, player, state::UiState};
use super::{detail, resources};

pub(crate) fn episode_title(episode: &crate::backend_api::FrontendEpisode) -> String {
    if episode.title_cn.trim().is_empty() {
        episode.title.trim().to_string()
    } else {
        episode.title_cn.trim().to_string()
    }
}

pub(crate) fn episode_subtitle(episode: &crate::backend_api::FrontendEpisode) -> String {
    let mut states = Vec::new();
    if episode.cached {
        states.push("本地可播放");
    } else {
        states.push("在线");
    }
    states.push(if episode.watched {
        "已观看"
    } else {
        "未观看"
    });
    if !episode.bgm_collection_label.trim().is_empty() {
        states.push(episode.bgm_collection_label.as_str());
    }
    states.join(" · ")
}

pub(crate) fn build_episode_list(state: &Rc<UiState>, subject: &FrontendSubject) -> gtk::ListView {
    let model = gio::ListStore::new::<glib::BoxedAnyObject>();
    for episode in subject.episodes_detail.iter().cloned() {
        model.append(&glib::BoxedAnyObject::new(episode));
    }
    let selection = gtk::NoSelection::new(Some(model));
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = state.clone();
    let subject_for_setup = subject.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        root.set_hexpand(true);
        root.add_css_class("nx-episode-row");
        let content = gtk::Button::new();
        content.set_has_frame(false);
        content.set_hexpand(true);
        content.set_halign(gtk::Align::Fill);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
        text.set_hexpand(true);
        text.append(&label("", "heading"));
        text.append(&label("", "dim-label"));
        content.set_child(Some(&text));
        root.append(&content);
        let watched = icon_button("emblem-ok-symbolic", "标记为已观看");
        watched.set_valign(gtk::Align::Center);
        root.append(&watched);
        list_item.set_child(Some(&root));

        let item_for_content = list_item.clone();
        let state_for_content = state_for_setup.clone();
        let subject_for_content = subject_for_setup.clone();
        content.connect_clicked(move |_| {
            if let Some(episode) = episode_from_list_item(&item_for_content) {
                activate_episode(&state_for_content, subject_for_content.clone(), episode);
            }
        });

        let item_for_watch = list_item.clone();
        let state_for_watch = state_for_setup.clone();
        let subject_id = subject_for_setup.subject_id;
        watched.connect_clicked(move |_| {
            let Some(episode) = episode_from_list_item(&item_for_watch) else {
                return;
            };
            if let Some(episode_id) = episode.bgm_episode_id
                && !episode.watched
                && subject_id > 0
            {
                mark_episode_watched(&state_for_watch, subject_id, episode_id);
            }
        });
    });
    factory.connect_bind(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(episode) = episode_from_list_item(list_item) else {
            return;
        };
        let Some(root) = list_item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(content) = root.first_child().and_downcast::<gtk::Button>() else {
            return;
        };
        let Some(text) = content.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(title) = text.first_child().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(subtitle) = title.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        title.set_text(&format!(
            "第 {} 集 · {}",
            episode.episode,
            episode_title(&episode)
        ));
        subtitle.set_text(&episode_subtitle(&episode));
        if let Some(watched) = root.last_child().and_downcast::<gtk::Button>() {
            watched.set_visible(episode.bgm_episode_id.is_some());
            watched.set_sensitive(episode.bgm_episode_id.is_some() && !episode.watched);
            watched.set_tooltip_text(Some(if episode.watched {
                "已观看"
            } else {
                "标记为已观看"
            }));
        }
    });
    let list = gtk::ListView::new(Some(selection), Some(factory));
    list.set_show_separators(true);
    list.set_vexpand(false);
    list.set_hexpand(true);
    list.add_css_class("nx-episode-list");
    list
}

pub(crate) fn episode_from_list_item(
    list_item: &gtk::ListItem,
) -> Option<crate::backend_api::FrontendEpisode> {
    list_item
        .item()
        .and_downcast::<glib::BoxedAnyObject>()
        .map(|object| {
            object
                .borrow::<crate::backend_api::FrontendEpisode>()
                .clone()
        })
}

pub(crate) fn activate_episode(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    episode: FrontendEpisode,
) {
    if episode.media_id.is_some() {
        player::open_player(state, subject, episode);
    } else {
        resources::open_resources(state, subject, episode.episode as f64);
    }
}

pub(crate) fn preferred_playback_episode(
    subject: &FrontendSubject,
) -> Option<crate::backend_api::FrontendEpisode> {
    subject
        .current_episode
        .and_then(|number| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| episode.episode == number && episode.media_id.is_some())
        })
        .or_else(|| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| !episode.watched && episode.media_id.is_some())
        })
        .or_else(|| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| episode.media_id.is_some())
        })
        .cloned()
}

pub(crate) fn refresh_detail(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
) {
    if !subject.local || subject.subject_id <= 0 {
        events::show_error(state, "在线条目无需刷新本地元数据".to_string());
        return;
    }
    if detail::start_detail_dynamic_refresh(state, subject, current_subject, container, true, true)
    {
        events::show_success(state, "已在后台刷新 Bangumi 评分和排名");
    }
}

pub(crate) fn sync_subject(state: &Rc<UiState>, subject_id: i64) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::sync_bangumi_subject(context, RefreshSubjectRequest { subject_id })
        },
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        events::show_success(&state, summary.message);
                        events::request_snapshot(&state);
                    }
                    Err(error) => events::show_error(&state, format!("Bangumi 同步失败：{error}")),
                }
            }
        },
    );
}

pub(crate) fn mark_episode_watched(state: &Rc<UiState>, subject_id: i64, episode_id: i64) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::update_bangumi_episode(
                context,
                BangumiUpdateEpisodeInput {
                    subject_id,
                    episode_id,
                    collection_type: 2,
                },
            )
        },
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        events::show_success(&state, summary.message);
                        events::request_snapshot(&state);
                    }
                    Err(error) => events::show_error(&state, format!("更新观看状态失败：{error}")),
                }
            }
        },
    );
}
