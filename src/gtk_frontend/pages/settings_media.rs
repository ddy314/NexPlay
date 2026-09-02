use super::super::components::*;
use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::settings_form;

pub(crate) fn append_media_group(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    preferences: &adw::PreferencesPage,
    media_group: &adw::PreferencesGroup,
) {
    let media_row = adw::ActionRow::new();
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
    settings_form::render_media_group(state, &form);
    preferences.add(media_group);
}
