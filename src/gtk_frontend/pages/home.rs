use super::super::components::*;
use super::super::prelude::*;
use super::super::{skeleton, state::UiState};

pub(crate) fn render_home(state: &Rc<UiState>) {
    clear_box(&state.home);
    state.home.append(&page_header(
        "主页",
        "在这里继续观看、回到最近打开的内容，或浏览本地片库。",
    ));
    let actions = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    let discover_button = action_button("打开发现", "compass-symbolic");
    let library_button = action_button("打开媒体库", "folder-videos-symbolic");
    let insights_button = action_button("观看洞察", "view-statistics-symbolic");
    actions.append(&discover_button);
    actions.append(&library_button);
    actions.append(&insights_button);
    state.home.append(&actions);
    {
        let state = state.clone();
        discover_button.connect_clicked(move |_| state.stack.set_visible_child_name("discover"));
    }
    {
        let state = state.clone();
        library_button.connect_clicked(move |_| state.stack.set_visible_child_name("library"));
    }
    {
        let state = state.clone();
        insights_button.connect_clicked(move |_| state.stack.set_visible_child_name("insights"));
    }
    if state.sync_loading.get() {
        let sync_progress = gtk::ProgressBar::new();
        sync_progress.set_fraction(state.sync_fraction.get());
        sync_progress.set_show_text(true);
        sync_progress.set_text(Some(&state.sync_message.borrow()));
        state.home.append(&sync_progress);
    }

    if let Some(feed) = state.home_feed.borrow().clone() {
        let mut has_items = false;
        for section in feed.sections {
            if section.items.is_empty() {
                continue;
            }
            has_items = true;
            let subjects = section
                .items
                .into_iter()
                .map(|item| item.subject)
                .collect::<Vec<_>>();
            state.home.append(&subject_shelf(
                state,
                &section.title,
                &section.subtitle,
                &subjects,
            ));
        }
        if !has_items {
            state.home.append(&status(
                "从第一部番剧开始",
                "在设置中添加媒体目录，然后从媒体库启动扫描。",
                "folder-videos-symbolic",
            ));
        }
    } else {
        if let Some(error) = state.home_feed_error.borrow().clone() {
            let error_page = status(
                "首页内容暂不可用",
                &format!("{error}。可以稍后重试。"),
                "dialog-warning-symbolic",
            );
            let retry = action_button("重试首页内容", "view-refresh-symbolic");
            let state_for_retry = state.clone();
            retry.connect_clicked(move |_| {
                state_for_retry.home_feed_error.replace(None);
                state_for_retry.home_feed_requested.set(false);
                render_home(&state_for_retry);
            });
            error_page.set_child(Some(&retry));
            state.home.append(&error_page);
        } else {
            state.home.append(&skeleton::home());
            if !state.snapshot_loading.get() {
                request_home_feed(state);
            }
        }
    }
}

pub(crate) fn request_home_feed(state: &Rc<UiState>) {
    if state.home_feed_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| home_feed(context),
        move |result: Result<HomeFeedResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.home_feed_requested.set(false);
            match result {
                Ok(feed) => {
                    state.home_feed_error.replace(None);
                    state.home_feed.replace(Some(feed));
                }
                Err(error) => {
                    state.home_feed_error.replace(Some(error));
                }
            }
            render_home(&state);
        },
    );
}
