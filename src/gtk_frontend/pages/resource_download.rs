use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, state::UiState};
use super::{downloads, insights};

use insights::format_bytes_i64;

pub(crate) fn prepare_resource(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    episode_number: f64,
    resource: crate::service::EpisodeResourceData,
) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::prepare_resource_download(
                context,
                PrepareResourceDownloadRequest {
                    resource,
                    subject_provider: subject.provider,
                    provider_subject_id: subject.provider_subject_id,
                    episode_number: Some(episode_number),
                },
            )
        },
        move |result: Result<PreparedResourceDownloadResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(prepared) => show_torrent_file_dialog(&state, prepared),
                Err(error) => events::show_error(&state, format!("准备下载失败：{error}")),
            }
        },
    );
}

pub(crate) fn show_torrent_file_dialog(
    state: &Rc<UiState>,
    prepared: PreparedResourceDownloadResponse,
) {
    let dialog = adw::Dialog::new();
    dialog.set_title("选择种子文件");
    dialog.set_content_width(680);
    dialog.set_content_height(560);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&label(&prepared.task.title, "title-2"));
    content.append(&label(
        "选择要下载的文件，然后确认开始任务。关闭对话框会取消预览任务。",
        "dim-label",
    ));
    let selected = Rc::new(RefCell::new(
        prepared
            .files
            .iter()
            .filter(|file| file.priority > 0)
            .map(|file| file.index)
            .collect::<HashSet<_>>(),
    ));
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    for file in prepared.files.iter() {
        let row = gtk::ListBoxRow::new();
        let check = gtk::CheckButton::with_label(&format!(
            "{} · {}",
            file.name,
            format_bytes_i64(file.size)
        ));
        check.set_active(selected.borrow().contains(&file.index));
        let selected_for_check = selected.clone();
        let index = file.index;
        check.connect_toggled(move |check| {
            if check.is_active() {
                selected_for_check.borrow_mut().insert(index);
            } else {
                selected_for_check.borrow_mut().remove(&index);
            }
        });
        row.set_child(Some(&check));
        list.append(&row);
    }
    content.append(&scrolled(&list));
    let actions = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    actions.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("取消");
    let confirm = action_button("确认下载", "folder-download-symbolic");
    confirm.add_css_class("suggested-action");
    actions.append(&cancel);
    actions.append(&confirm);
    content.append(&actions);
    dialog.set_child(Some(&content));
    let committed = Rc::new(Cell::new(false));
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let dialog = dialog.clone();
        let state = state.clone();
        let selected = selected.clone();
        let task_id = prepared.task.id;
        let committed = committed.clone();
        confirm.connect_clicked(move |_| {
            let indexes = selected.borrow().iter().copied().collect::<Vec<_>>();
            if indexes.is_empty() {
                events::show_error(&state, "至少选择一个种子文件".to_string());
                return;
            }
            committed.set(true);
            let weak = Rc::downgrade(&state);
            state.runtime.submit(
                move |context| {
                    confirm_resource_download(
                        context,
                        ConfirmResourceDownloadRequest {
                            task_id,
                            selected_file_indexes: indexes,
                        },
                    )
                },
                move |result: Result<crate::service::DownloadTaskData, String>| {
                    if let Some(state) = weak.upgrade() {
                        match result {
                            Ok(_) => {
                                events::show_success(&state, "下载任务已开始");
                                state.downloads_data.replace(None);
                                state.downloads_requested.set(false);
                                downloads::request_downloads(&state);
                            }
                            Err(error) => {
                                events::show_error(&state, format!("确认下载失败：{error}"))
                            }
                        }
                    }
                },
            );
            dialog.close();
        });
    }
    {
        let state = state.clone();
        let committed = committed.clone();
        let task_id = prepared.task.id;
        dialog.connect_closed(move |_| {
            if committed.get() {
                return;
            }
            downloads::control_download(&state, task_id, "cancel", false);
        });
    }
    dialog.present(Some(&state.window));
}
