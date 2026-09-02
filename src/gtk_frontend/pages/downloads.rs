use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, skeleton, state::UiState};
use super::insights::format_bytes_i64;

pub(crate) fn render_downloads(state: &Rc<UiState>) {
    clear_box(&state.downloads);
    state.downloads.append(&page_header(
        "下载",
        "在这里查看任务进度，暂停、取消或移除下载。",
    ));
    if let Some(data) = state.downloads_data.borrow().clone() {
        if data.tasks.is_empty() {
            state.downloads.append(&status(
                "暂无下载任务",
                "从条目详情打开资源搜索，然后选择 Nyaa 资源加入 qBittorrent。",
                "folder-download-symbolic",
            ));
        } else {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
            list.add_css_class("nx-download-list");
            for task in data.tasks {
                let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
                body.set_hexpand(true);
                body.add_css_class("nx-download-row");
                let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                heading.set_hexpand(true);
                let title = label(&task.title, "heading");
                title.set_hexpand(true);
                title.set_wrap(true);
                heading.append(&title);
                let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                actions.set_valign(gtk::Align::Center);
                if matches!(task.status.as_str(), "paused" | "queued" | "downloading") {
                    let action = if task.status == "paused" {
                        "resume"
                    } else {
                        "pause"
                    };
                    let button = icon_button(
                        if action == "pause" {
                            "media-playback-pause-symbolic"
                        } else {
                            "media-playback-start-symbolic"
                        },
                        if action == "pause" {
                            "暂停"
                        } else {
                            "继续"
                        },
                    );
                    let state_for_action = state.clone();
                    let id = task.id;
                    let action_name = action.to_string();
                    button.connect_clicked(move |_| {
                        control_download(&state_for_action, id, &action_name, false)
                    });
                    actions.append(&button);
                    let cancel = icon_button("process-stop-symbolic", "取消下载");
                    let state_for_cancel = state.clone();
                    let id = task.id;
                    cancel.connect_clicked(move |_| confirm_cancel_download(&state_for_cancel, id));
                    actions.append(&cancel);
                }
                let remove = icon_button("user-trash-symbolic", "删除任务");
                let state_for_remove = state.clone();
                let id = task.id;
                remove.connect_clicked(move |_| confirm_remove_download(&state_for_remove, id));
                actions.append(&remove);
                heading.append(&actions);
                body.append(&heading);
                body.append(&label(
                    &format!(
                        "{} · {} · {} / {} · 速度 {} /s",
                        task.status,
                        if task.stale {
                            "状态过期"
                        } else {
                            "qBittorrent"
                        },
                        format_bytes_i64(task.downloaded),
                        format_bytes_i64(task.size),
                        format_bytes_i64(task.dlspeed),
                    ),
                    "dim-label",
                ));
                let progress_line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let progress = gtk::ProgressBar::new();
                progress.set_fraction(task.progress.clamp(0.0, 1.0));
                progress.set_hexpand(true);
                progress.set_show_text(false);
                progress.set_valign(gtk::Align::Center);
                let percent = label(format!("{:.0}%", task.progress * 100.0), "dim-label");
                percent.set_width_chars(5);
                percent.set_xalign(1.0);
                progress_line.append(&progress);
                progress_line.append(&percent);
                body.append(&progress_line);
                list.append(&body);
            }
            state.downloads.append(&list);
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
            state_for_retry.downloads_error.replace(None);
            state_for_retry.downloads_requested.set(false);
            render_downloads(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.downloads.append(&error_page);
    } else {
        state.downloads.append(&skeleton::downloads());
        request_downloads(state);
    }
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
