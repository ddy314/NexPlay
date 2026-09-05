use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, skeleton, state::UiState};
use super::insights::format_bytes_i64;
use crate::service::DownloadTaskData;

pub(crate) fn render_downloads(state: &Rc<UiState>) {
    clear_box(&state.downloads);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 18);
    body.set_hexpand(true);
    body.set_vexpand(false);
    let clamp = adw::Clamp::builder()
        .maximum_size(900)
        .tightening_threshold(640)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .child(&body)
        .build();
    state.downloads.append(&clamp);
    body.append(&download_header(state));
    if let Some(data) = state.downloads_data.borrow().clone() {
        if data.tasks.is_empty() {
            body.append(&status(
                "暂无下载任务",
                "从条目详情打开资源搜索，然后选择 Nyaa 资源加入 qBittorrent。",
                "folder-download-symbolic",
            ));
        } else {
            body.append(&download_overview(&data.tasks));
            if let Some(error) = state.downloads_error.borrow().clone() {
                body.append(&refresh_banner(state, &error));
            }
            append_task_group(
                &body,
                state,
                "处理中",
                &format!("{} 个任务正在排队、下载或暂停", count_active(&data.tasks)),
                data.tasks.iter().filter(|task| is_active(task)).collect(),
            );
            append_task_group(
                &body,
                state,
                "需要处理",
                &format!("{} 个任务需要检查或清理", count_attention(&data.tasks)),
                data.tasks
                    .iter()
                    .filter(|task| needs_attention(task))
                    .collect(),
            );
            append_task_group(
                &body,
                state,
                "已完成",
                &format!("{} 个任务已经完成", count_completed(&data.tasks)),
                data.tasks
                    .iter()
                    .filter(|task| task.status == "completed")
                    .collect(),
            );
        }
    } else if let Some(error) = state.downloads_error.borrow().clone() {
        let error_page = status(
            "下载状态暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试下载状态", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            retry_downloads(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        body.append(&error_page);
    } else {
        body.append(&skeleton::downloads());
        request_downloads(state);
    }
}

fn download_header(state: &Rc<UiState>) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_hexpand(true);
    let copy = page_header("下载", "管理 qBittorrent 中的任务，查看进度并处理异常。");
    copy.set_hexpand(true);
    header.append(&copy);

    let refresh = icon_button("view-refresh-symbolic", "刷新下载状态");
    refresh.set_valign(gtk::Align::Start);
    refresh.set_sensitive(!state.downloads_requested.get());
    let state_for_refresh = state.clone();
    refresh.connect_clicked(move |_| {
        request_downloads(&state_for_refresh);
        render_downloads(&state_for_refresh);
    });
    header.append(&refresh);
    header
}

fn download_overview(tasks: &[DownloadTaskData]) -> adw::WrapBox {
    let overview = adw::WrapBox::builder()
        .child_spacing(12)
        .line_spacing(12)
        .line_homogeneous(true)
        .natural_line_length(960)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    overview.set_hexpand(true);
    overview.set_halign(gtk::Align::Fill);
    overview.set_vexpand(false);
    overview.set_justify_last_line(false);

    let speed = tasks
        .iter()
        .filter(|task| is_active(task))
        .map(|task| task.dlspeed.max(0))
        .sum::<i64>();
    for (icon, title, value) in [
        (
            "folder-download-symbolic",
            "处理中",
            count_active(tasks).to_string(),
        ),
        (
            "emblem-ok-symbolic",
            "已完成",
            count_completed(tasks).to_string(),
        ),
        (
            "dialog-warning-symbolic",
            "需处理",
            count_attention(tasks).to_string(),
        ),
        (
            "network-transmit-receive-symbolic",
            "当前速度",
            format!("{}/s", format_bytes_i64(speed)),
        ),
    ] {
        overview.append(&download_stat(icon, title, &value));
    }
    overview
}

fn download_stat(icon_name: &str, title: &str, value: &str) -> gtk::Frame {
    let card = gtk::Frame::new(None);
    card.add_css_class("card");
    card.add_css_class("nx-download-stat");
    card.set_hexpand(true);
    card.set_vexpand(false);
    card.set_valign(gtk::Align::Start);
    card.set_height_request(72);
    card.set_width_request(180);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    content.add_css_class("nx-download-stat-content");
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    icon.add_css_class("dim-label");
    heading.append(&icon);
    heading.append(&label(title, "dim-label"));
    content.append(&heading);

    let value_label = label(value, "title-2");
    value_label.set_wrap(false);
    value_label.add_css_class("numeric");
    content.append(&value_label);
    card.set_child(Some(&content));
    card
}

fn append_task_group(
    container: &gtk::Box,
    state: &Rc<UiState>,
    title: &str,
    description: &str,
    tasks: Vec<&DownloadTaskData>,
) {
    if tasks.is_empty() {
        return;
    }
    let group = adw::PreferencesGroup::new();
    group.set_title(title);
    group.set_description(Some(description));
    group.set_hexpand(true);
    for task in tasks {
        group.add(&download_task_row(state, task));
    }
    container.append(&group);
}

fn download_task_row(state: &Rc<UiState>, task: &DownloadTaskData) -> adw::PreferencesRow {
    let row = adw::PreferencesRow::new();
    row.set_selectable(false);
    row.add_css_class("nx-download-task-row");

    let content = gtk::Overlay::new();
    content.set_hexpand(true);
    content.set_height_request(72);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_hexpand(true);
    header.set_height_request(72);
    header.set_margin_start(16);
    header.set_margin_end(16);

    let icon = gtk::Box::new(gtk::Orientation::Vertical, 0);
    icon.set_width_request(32);
    icon.set_height_request(32);
    icon.set_halign(gtk::Align::Start);
    icon.set_valign(gtk::Align::Center);
    icon.add_css_class("nx-download-status-icon");
    let icon_image = gtk::Image::from_icon_name(status_icon(task));
    icon_image.set_pixel_size(18);
    icon_image.set_halign(gtk::Align::Center);
    icon_image.set_valign(gtk::Align::Center);
    icon_image.add_css_class(status_css_class(task));
    icon.append(&icon_image);
    header.append(&icon);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    copy.set_valign(gtk::Align::Center);
    let title = label(&task.title, "title-4");
    title.set_hexpand(true);
    title.set_wrap(false);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_tooltip_text(Some(&task.title));
    copy.append(&title);
    let summary = label(&task_summary(task), "dim-label");
    summary.set_hexpand(true);
    summary.set_wrap(false);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if !task.error.trim().is_empty() {
        summary.set_tooltip_text(Some(task.error.trim()));
    }
    copy.append(&summary);
    header.append(&copy);
    header.append(&task_actions(state, task));

    let progress = gtk::ProgressBar::new();
    progress.set_fraction(task.progress.clamp(0.0, 1.0));
    progress.set_show_text(false);
    progress.set_height_request(3);
    progress.set_margin_start(16);
    progress.set_margin_end(16);
    progress.set_margin_bottom(8);
    progress.set_halign(gtk::Align::Fill);
    progress.set_valign(gtk::Align::End);
    progress.set_hexpand(true);
    progress.add_css_class("nx-download-progress");
    content.set_child(Some(&header));
    content.add_overlay(&progress);
    row.set_child(Some(&content));
    row
}

fn task_actions(state: &Rc<UiState>, task: &DownloadTaskData) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    actions.set_valign(gtk::Align::Center);

    if is_active(task) {
        let paused = task.status == "paused";
        let action_name = if paused { "resume" } else { "pause" };
        let action = icon_button(
            if paused {
                "media-playback-start-symbolic"
            } else {
                "media-playback-pause-symbolic"
            },
            if paused {
                "继续下载"
            } else {
                "暂停下载"
            },
        );
        if paused {
            action.add_css_class("suggested-action");
        }
        let state_for_action = state.clone();
        let task_id = task.id;
        let action_name = action_name.to_string();
        action.connect_clicked(move |_| {
            control_download(&state_for_action, task_id, &action_name, false)
        });
        actions.append(&action);
    }

    let menu_button = gtk::MenuButton::new();
    menu_button.set_icon_name("view-more-symbolic");
    menu_button.set_tooltip_text(Some("更多操作"));
    menu_button.add_css_class("flat");
    let menu = gio::Menu::new();
    let action_group = gio::SimpleActionGroup::new();
    if is_active(task) {
        menu.append(Some("取消任务…"), Some("download.cancel"));
        let cancel = gio::SimpleAction::new("cancel", None);
        let state_for_cancel = state.clone();
        let task_id = task.id;
        cancel.connect_activate(move |_, _| {
            confirm_cancel_download(&state_for_cancel, task_id);
        });
        action_group.add_action(&cancel);
    }
    menu.append(Some("删除任务…"), Some("download.remove"));
    let remove = gio::SimpleAction::new("remove", None);
    let state_for_remove = state.clone();
    let task_id = task.id;
    remove.connect_activate(move |_, _| {
        confirm_remove_download(&state_for_remove, task_id);
    });
    action_group.add_action(&remove);
    menu_button.set_menu_model(Some(&menu));
    menu_button.insert_action_group("download", Some(&action_group));
    actions.append(&menu_button);
    actions
}

fn refresh_banner(state: &Rc<UiState>, error: &str) -> adw::Banner {
    let banner = adw::Banner::new(&format!("下载状态刷新失败：{error}"));
    banner.set_use_markup(false);
    banner.set_button_label(Some("重试"));
    banner.set_revealed(true);
    let state_for_retry = state.clone();
    banner.connect_button_clicked(move |_| retry_downloads(&state_for_retry));
    banner
}

fn retry_downloads(state: &Rc<UiState>) {
    state.downloads_error.replace(None);
    state.downloads_requested.set(false);
    request_downloads(state);
    render_downloads(state);
}

fn task_summary(task: &DownloadTaskData) -> String {
    let mut parts = vec![
        status_label(task).to_string(),
        format_percent(task.progress),
    ];
    if let Some(episode) = task.episode_number {
        parts.push(format!("第 {} 集", format_episode_number(episode)));
    }
    if !task.subject_provider.trim().is_empty() {
        parts.push(task.subject_provider.trim().to_string());
    }
    let transfer = task_transfer_summary(task);
    if !transfer.is_empty() {
        parts.push(transfer);
    }
    parts.join(" · ")
}

fn task_transfer_summary(task: &DownloadTaskData) -> String {
    if task.status == "completed" {
        return String::new();
    }
    let mut parts = Vec::new();
    if task.size > 0 {
        parts.push(format!(
            "{} / {}",
            format_bytes_i64(task.downloaded),
            format_bytes_i64(task.size)
        ));
    } else if task.downloaded > 0 {
        parts.push(format!("已下载 {}", format_bytes_i64(task.downloaded)));
    }
    if task.dlspeed > 0 {
        parts.push(format!("{} /s", format_bytes_i64(task.dlspeed)));
    }
    if let Some(eta) = format_eta(task.eta) {
        parts.push(format!("剩余 {eta}"));
    }
    if parts.is_empty() {
        match task.status.as_str() {
            "paused" => parts.push("任务已暂停".to_string()),
            status if status == "failed" || status == "missing" => {
                parts.push("需要检查任务状态".to_string())
            }
            _ => parts.push("等待 qBittorrent 返回状态".to_string()),
        }
    }
    parts.join(" · ")
}

fn format_eta(seconds: i64) -> Option<String> {
    if seconds < 0 || seconds >= 8_640_000 {
        return None;
    }
    if seconds == 0 {
        return Some("即将完成".to_string());
    }
    if seconds < 60 {
        return Some("不到 1 分钟".to_string());
    }
    if seconds < 3_600 {
        return Some(format!("{} 分钟", (seconds + 59) / 60));
    }
    if seconds < 86_400 {
        return Some(format!(
            "{} 小时 {} 分钟",
            seconds / 3_600,
            (seconds % 3_600 + 59) / 60
        ));
    }
    Some(format!("{} 天", (seconds + 86_399) / 86_400))
}

fn format_percent(progress: f64) -> String {
    format!("{:.0}%", progress.clamp(0.0, 1.0) * 100.0)
}

fn format_episode_number(number: f64) -> String {
    if number.fract().abs() < f64::EPSILON {
        format!("{}", number as i64)
    } else {
        format!("{number:.1}")
    }
}

fn status_label(task: &DownloadTaskData) -> &'static str {
    if task.stale {
        return "状态未同步";
    }
    match task.status.as_str() {
        "downloading" => "下载中",
        "queued" => "排队中",
        "paused" => "已暂停",
        "completed" => "已完成",
        "failed" => "下载失败",
        "missing" => "任务已丢失",
        "cancelled" => "已取消",
        "pending" => "准备中",
        _ => "等待中",
    }
}

fn status_icon(task: &DownloadTaskData) -> &'static str {
    if task.stale {
        return "dialog-warning-symbolic";
    }
    match task.status.as_str() {
        "completed" => "emblem-ok-symbolic",
        "failed" => "dialog-error-symbolic",
        "missing" | "cancelled" => "dialog-warning-symbolic",
        "paused" => "media-playback-pause-symbolic",
        "queued" | "pending" => "view-refresh-symbolic",
        _ => "folder-download-symbolic",
    }
}

fn status_css_class(task: &DownloadTaskData) -> &'static str {
    if task.stale || matches!(task.status.as_str(), "missing" | "cancelled") {
        "warning"
    } else if task.status == "failed" {
        "error"
    } else if task.status == "completed" {
        "success"
    } else {
        "accent"
    }
}

fn is_active(task: &DownloadTaskData) -> bool {
    !task.stale
        && !matches!(
            task.status.as_str(),
            "completed" | "failed" | "missing" | "cancelled"
        )
}

fn needs_attention(task: &DownloadTaskData) -> bool {
    task.stale || matches!(task.status.as_str(), "failed" | "missing" | "cancelled")
}

fn count_active(tasks: &[DownloadTaskData]) -> usize {
    tasks.iter().filter(|task| is_active(task)).count()
}

fn count_attention(tasks: &[DownloadTaskData]) -> usize {
    tasks.iter().filter(|task| needs_attention(task)).count()
}

fn count_completed(tasks: &[DownloadTaskData]) -> usize {
    tasks
        .iter()
        .filter(|task| task.status == "completed")
        .count()
}

pub(crate) fn request_downloads(state: &Rc<UiState>) {
    if state.downloads_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| download_tasks(context),
        move |result: Result<DownloadTasksResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.downloads_requested.set(false);
            match result {
                Ok(data) => {
                    state.downloads_error.replace(None);
                    state.downloads_data.replace(Some(data));
                }
                Err(error) => {
                    state.downloads_error.replace(Some(error));
                }
            };
            render_downloads(&state);
        },
    );
}

pub(crate) fn control_download(
    state: &Rc<UiState>,
    task_id: i64,
    action: &str,
    delete_files: bool,
) {
    let weak = Rc::downgrade(state);
    let action = action.to_string();
    state.runtime.submit(
        move |context| {
            crate::backend_api::control_download_task(
                context,
                DownloadTaskActionRequest {
                    task_id,
                    action,
                    delete_files,
                },
            )
        },
        move |result: Result<DownloadTasksResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(data) => {
                    state.downloads_data.replace(Some(data));
                }
                Err(error) => events::show_error(&state, format!("下载操作失败：{error}")),
            };
            render_downloads(&state);
        },
    );
}

pub(crate) fn confirm_remove_download(state: &Rc<UiState>, task_id: i64) {
    let dialog = adw::AlertDialog::new(
        Some("删除下载任务？"),
        Some("这会删除 NexPlay 的任务记录；是否同时删除 qBittorrent 中的文件由下一步确认。"),
    );
    dialog.add_response("cancel", "取消");
    dialog.add_response("remove", "仅移除记录");
    dialog.add_response("delete", "删除任务和文件");
    dialog.set_default_response(Some("remove"));
    dialog.set_close_response("cancel");
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| match response {
        "remove" => control_download(&state_for_callback, task_id, "remove", false),
        "delete" => control_download(&state_for_callback, task_id, "cancel", true),
        _ => {}
    });
    dialog.present(Some(&state.window));
}

pub(crate) fn confirm_cancel_download(state: &Rc<UiState>, task_id: i64) {
    let dialog = adw::AlertDialog::new(
        Some("取消下载任务？"),
        Some("这会停止 qBittorrent 中的任务并移除 NexPlay 的任务记录，但不会删除已下载文件。"),
    );
    dialog.add_response("keep", "继续下载");
    dialog.add_response("cancel", "取消任务");
    dialog.set_default_response(Some("keep"));
    dialog.set_close_response("keep");
    dialog.set_response_appearance("cancel", adw::ResponseAppearance::Destructive);
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "cancel" {
            control_download(&state_for_callback, task_id, "cancel", false);
        }
    });
    dialog.present(Some(&state.window));
}
