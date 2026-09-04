use super::super::components::*;
use super::super::events;
use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::settings_form;

pub(crate) fn append_media_group(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    preferences: &adw::PreferencesPage,
    media_group: &adw::PreferencesGroup,
) {
    let media_row = action_row();
    media_row.set_title("媒体目录");
    media_row.set_subtitle("扫描这些文件夹中的视频文件");
    let add_folder = icon_button("folder-new-symbolic", "添加媒体目录");
    media_row.add_suffix(&add_folder);
    media_group.add(&media_row);
    {
        let state = state.clone();
        let form = form.clone();
        add_folder.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            dialog.set_title("选择媒体目录");
            let parent = state.window.clone();
            let state = state.clone();
            let form = form.clone();
            dialog.select_folder(Some(&parent), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    let value = path.display().to_string();
                    if !form
                        .media_libraries
                        .borrow()
                        .iter()
                        .any(|item| item == &value)
                    {
                        form.media_libraries.borrow_mut().push(value);
                        settings_form::settings_changed(&state);
                        settings_form::render_media_group(&state, &form);
                    }
                }
            });
        });
    }

    let scan_row = action_row();
    scan_row.set_title("扫描媒体库");
    scan_row.set_subtitle("扫描已配置目录并更新本地媒体条目");
    let scan_button = gtk::Button::with_label("开始扫描");
    scan_button.set_sensitive(!state.scan_loading.get());
    scan_row.add_suffix(&scan_button);
    media_group.add(&scan_row);
    settings_form::render_media_group(state, &form);
    {
        let state = state.clone();
        let button = scan_button.clone();
        scan_button.connect_clicked(move |_| start_scan(&state, &button));
    }
    preferences.add(media_group);
}

fn start_scan(state: &Rc<UiState>, button: &gtk::Button) {
    if state.scan_loading.replace(true) {
        return;
    }
    button.set_label("扫描中…");
    button.set_sensitive(false);
    state.scan_fraction.set(0.0);
    state.scan_message.replace("扫描已排队…".to_string());
    let weak = Rc::downgrade(state);
    let button = button.clone();
    state.runtime.submit(
        |context| scan(context),
        move |result: Result<ScanResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.scan_loading.set(false);
            button.set_label("开始扫描");
            button.set_sensitive(true);
            match result {
                Ok(response) => {
                    state.snapshot.replace(response.snapshot);
                    state.scan_message.replace(format!(
                        "扫描完成：新增 {}，修改 {}，删除 {}",
                        response.summary.added, response.summary.modified, response.summary.deleted
                    ));
                    events::show_success(&state, "媒体库扫描完成");
                    events::render_all(&state);
                }
                Err(error) => {
                    events::show_error(&state, format!("扫描失败：{error}"));
                }
            }
        },
    );
}
