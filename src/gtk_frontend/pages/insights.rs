use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, state::UiState};

pub(crate) fn render_insights(state: &Rc<UiState>) {
    clear_box(&state.insights);
    state.insights.append(&page_header(
        "观看洞察",
        "历史会话和标签统计保留在现有数据库中；GTK 阶段不会产生新的播放会话。",
    ));
    let range = gtk::DropDown::from_strings(&["本周", "本月", "本年"]);
    range.set_selected(match state.insight_range.get() {
        InsightRange::Week => 0,
        InsightRange::Month => 1,
        InsightRange::Year => 2,
    });
    state.insights.append(&range);
    range.connect_selected_notify({
        let state = state.clone();
        move |dropdown| {
            state.insight_range.set(match dropdown.selected() {
                1 => InsightRange::Month,
                2 => InsightRange::Year,
                _ => InsightRange::Week,
            });
            state.insights_data.replace(None);
            state.insights_error.replace(None);
            state.insights_requested.set(false);
            render_insights(&state);
        }
    });
    if let Some(data) = state.insights_data.borrow().clone() {
        let metrics = adw::WrapBox::builder()
            .child_spacing(12)
            .line_spacing(12)
            .line_homogeneous(true)
            .build();
        for (title, value) in [
            ("观看分钟", format!("{:.0}", data.total_minutes)),
            ("完成集数", data.completed_episodes.to_string()),
            ("活跃天数", data.active_days.to_string()),
            ("连续天数", data.streak_days.to_string()),
        ] {
            let group = adw::ActionRow::new();
            group.set_title(title);
            group.set_subtitle(&value);
            group.set_hexpand(true);
            metrics.append(&group);
        }
        state.insights.append(&metrics);
        for ring in data.rings {
            let row = adw::ActionRow::new();
            row.set_title(&ring.label);
            row.set_subtitle(&format!(
                "{:.1} / {:.1} {}",
                ring.value, ring.goal, ring.unit
            ));
            let progress = gtk::ProgressBar::new();
            progress.set_fraction(if ring.goal > 0.0 {
                (ring.value / ring.goal).clamp(0.0, 1.0)
            } else {
                0.0
            });
            progress.set_width_request(180);
            row.add_suffix(&progress);
            state.insights.append(&row);
        }
        if !data.daily.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("每日节奏");
            for point in data.daily.iter().take(14) {
                let row = adw::ActionRow::new();
                row.set_title(&point.label);
                row.set_subtitle(&format!("{:.1}", point.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.dayparts.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("时间分布");
            for point in data.dayparts.iter().take(8) {
                let row = adw::ActionRow::new();
                row.set_title(&point.label);
                row.set_subtitle(&format!("{:.1} 分钟", point.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.tags.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("标签分布");
            for tag in data.tags.iter().take(12) {
                let row = adw::ActionRow::new();
                row.set_title(&tag.label);
                row.set_subtitle(&format!("{:.1}", tag.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.highlights.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("亮点");
            for highlight in data.highlights.iter().take(8) {
                let row = adw::ActionRow::new();
                row.set_title(&highlight.title);
                row.set_subtitle(&highlight.detail);
                group.add(&row);
            }
            state.insights.append(&group);
        }
        let clear = action_button("清除本地播放历史", "user-trash-symbolic");
        let state_for_clear = state.clone();
        clear.connect_clicked(move |_| confirm_clear_insights(&state_for_clear));
        state.insights.append(&clear);
    } else if let Some(error) = state.insights_error.borrow().clone() {
        let error_page = status(
            "洞察暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试洞察", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.insights_error.replace(None);
            state_for_retry.insights_requested.set(false);
            render_insights(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.insights.append(&error_page);
    } else {
        state.insights.append(&status(
            "正在计算洞察",
            "本地统计会在后台读取现有记录。",
            "view-statistics-symbolic",
        ));
        request_insights(state);
    }
}

pub(crate) fn request_insights(state: &Rc<UiState>) {
    if state.insights_requested.replace(true) {
        return;
    }
    let range = state.insight_range.get();
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| insights_dashboard(context, InsightsDashboardRequest { range }),
        move |result: Result<InsightsDashboardResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.insights_requested.set(false);
            match result {
                Ok(data) => {
                    state.insights_error.replace(None);
                    state.insights_data.replace(Some(data));
                }
                Err(error) => {
                    state.insights_error.replace(Some(error));
                }
            }
            render_insights(&state);
        },
    );
}

pub(crate) fn confirm_clear_insights(state: &Rc<UiState>) {
    let dialog = adw::AlertDialog::new(
        Some("清除播放历史？"),
        Some("这只会清除本地播放分析记录，不会删除媒体、Bangumi 状态或下载任务。"),
    );
    dialog.add_response("cancel", "取消");
    dialog.add_response("clear", "清除");
    dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "clear" {
            return;
        }
        let weak = Rc::downgrade(&state_for_callback);
        state_for_callback.runtime.submit(
            crate::backend_api::clear_playback_analytics,
            move |result: Result<(), String>| {
                if let Some(state) = weak.upgrade() {
                    match result {
                        Ok(()) => {
                            state.insights_data.replace(None);
                            state.insights_requested.set(false);
                            events::show_success(&state, "播放历史已清除");
                            render_insights(&state);
                        }
                        Err(error) => events::show_error(&state, format!("清除失败：{error}")),
                    }
                }
            },
        );
    });
    dialog.present(Some(&state.window));
}

pub(crate) fn format_bytes_i64(value: i64) -> String {
    if value < 1024 {
        return format!("{value} B");
    }
    let units = ["KiB", "MiB", "GiB", "TiB"];
    let mut value = value as f64;
    for unit in units {
        value /= 1024.0;
        if value < 1024.0 {
            return format!("{value:.1} {unit}");
        }
    }
    format!("{value:.1} PiB")
}
