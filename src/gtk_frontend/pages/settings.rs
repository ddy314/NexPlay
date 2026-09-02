use super::super::components::*;
use super::super::prelude::*;
use super::super::skeleton;
use super::super::state::{SettingsForm, UiState};
use super::{
    settings_actions, settings_bangumi, settings_form, settings_integrations, settings_media,
    settings_preferences,
};

pub(crate) fn render_settings(state: &Rc<UiState>) {
    if state.settings_form.borrow().is_some() {
        return;
    }
    clear_box(&state.settings);
    state.settings.append(&page_header(
        "设置",
        "在这里管理媒体来源、服务连接和观看体验；修改会立即生效并自动保存。",
    ));
    let Some(settings) = state.settings_data.borrow().clone() else {
        if let Some(error) = state.settings_error.borrow().clone() {
            let error_page = status(
                "设置暂不可用",
                &format!("{error}。可以稍后重试。"),
                "dialog-warning-symbolic",
            );
            let retry = action_button("重试读取设置", "view-refresh-symbolic");
            let state_for_retry = state.clone();
            retry.connect_clicked(move |_| {
                state_for_retry.settings_error.replace(None);
                state_for_retry.settings_requested.set(false);
                render_settings(&state_for_retry);
            });
            error_page.set_child(Some(&retry));
            state.settings.append(&error_page);
            return;
        }
        state.settings.append(&skeleton::settings());
        request_settings(state);
        return;
    };
    build_settings_page(state, settings);
}

pub(crate) fn request_settings(state: &Rc<UiState>) {
    if state.settings_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| settings_config(context),
        move |result: Result<FrontendEditableSettings, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.settings_requested.set(false);
            match result {
                Ok(settings) => {
                    state.settings_error.replace(None);
                    settings_actions::apply_runtime_settings(
                        &settings.theme,
                        settings.reduced_motion,
                    );
                    state.settings_data.replace(Some(settings));
                }
                Err(error) => {
                    state.settings_error.replace(Some(error));
                }
            };
            render_settings(&state);
        },
    );
}

pub(crate) fn build_settings_page(state: &Rc<UiState>, settings: FrontendEditableSettings) {
    let controls = RefCell::new(std::collections::HashMap::new());
    let media_libraries = Rc::new(RefCell::new(settings.media_libraries.clone()));
    let media_group = adw::PreferencesGroup::new();
    media_group.set_title("媒体来源");
    let form = Rc::new(SettingsForm {
        base: settings.clone(),
        media_libraries,
        controls,
        secret_values: RefCell::new(HashMap::new()),
        media_group: media_group.clone(),
    });
    state.settings_form.replace(Some(form.clone()));

    let preferences = adw::PreferencesPage::new();
    preferences.set_vexpand(true);

    settings_media::append_media_group(state, &form, &preferences, &media_group);
    settings_bangumi::append_bangumi_group(state, &form, &preferences, &settings);
    settings_integrations::append_integrations(state, &form, &preferences, &settings);
    settings_preferences::append_preferences(state, &form, &preferences, &settings);

    settings_form::install_settings_focus_behavior(&preferences);
    state.settings.append(&preferences);
}
