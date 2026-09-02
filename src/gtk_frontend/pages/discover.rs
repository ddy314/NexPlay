use super::super::components::*;
use super::super::prelude::*;
use super::super::{skeleton, state::UiState};

pub(crate) fn render_discover(state: &Rc<UiState>) {
    clear_box(&state.discover);
    state
        .discover
        .append(&page_header("发现", "在这里浏览今日放送和正在上升的作品。"));
    if let Some(feed) = state.discovery_feed.borrow().clone() {
        if feed.today.is_empty() && feed.trending.is_empty() {
            state.discover.append(&status(
                "暂时没有发现内容",
                "公开日历为空或网络暂时不可用，本地媒体库仍然可以正常使用。",
                "compass-symbolic",
            ));
            return;
        }
        if !feed.today.is_empty() {
            state.discover.append(&subject_shelf(
                state,
                "今日放送",
                "Bangumi 每日放送",
                &feed.today,
            ));
        }
        if !feed.trending.is_empty() {
            state.discover.append(&subject_shelf(
                state,
                "正在上升",
                "公开收藏数、评分与排名综合排序",
                &feed.trending,
            ));
        }
    } else if let Some(error) = state.discovery_feed_error.borrow().clone() {
        let error_page = status(
            "发现内容暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试发现", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.discovery_feed_error.replace(None);
            state_for_retry.discovery_requested.set(false);
            render_discover(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.discover.append(&error_page);
    } else {
        state.discover.append(&skeleton::home());
        request_discovery(state);
    }
}

pub(crate) fn request_discovery(state: &Rc<UiState>) {
    if state.discovery_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| discovery_feed(context),
        move |result: Result<DiscoveryFeedResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.discovery_requested.set(false);
            match result {
                Ok(feed) => {
                    state.discovery_feed_error.replace(None);
                    state.discovery_feed.replace(Some(feed));
                }
                Err(error) => {
                    state.discovery_feed_error.replace(Some(error));
                }
            }
            render_discover(&state);
        },
    );
}
