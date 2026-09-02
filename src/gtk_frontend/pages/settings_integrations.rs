use super::super::components::*;
use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::{settings_actions, settings_form};

pub(crate) fn append_integrations(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    preferences: &adw::PreferencesPage,
    settings: &FrontendEditableSettings,
) {
    let dandan = adw::PreferencesGroup::new();
    dandan.set_title("DanDanPlay");
    settings_form::add_entry_control(
        state,
        &form,
        &dandan,
        "dandanplay_app_id",
        "App ID",
        &settings.dandanplay_app_id,
        false,
    );
    settings_form::add_secret_control(
        state,
        &form,
        &dandan,
        "dandanplay_app_secret",
        "App Secret",
        settings.dandanplay_app_secret_configured,
    );
    settings_form::add_secret_control(
        state,
        &form,
        &dandan,
        "dandanplay_api_key",
        "API Key",
        settings.dandanplay_api_key_configured,
    );
    preferences.add(&dandan);

    let nyaa = adw::PreferencesGroup::new();
    nyaa.set_title("Nyaa 资源搜索");
    settings_form::add_switch_control(
        state,
        &form,
        &nyaa,
        "nyaa_enabled",
        "启用 Nyaa",
        "详情页资源搜索使用 Nyaa RSS",
        settings.nyaa_enabled,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &nyaa,
        "nyaa_base_url",
        "服务地址",
        &settings.nyaa_base_url,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &nyaa,
        "nyaa_category",
        "分类",
        &settings.nyaa_category,
        false,
    );
    preferences.add(&nyaa);

    let qbit = adw::PreferencesGroup::new();
    qbit.set_title("qBittorrent 下载");
    settings_form::add_switch_control(
        state,
        &form,
        &qbit,
        "qbittorrent_enabled",
        "启用 qBittorrent",
        "连接下载服务以管理资源",
        settings.qbittorrent_enabled,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_base_url",
        "服务地址",
        &settings.qbittorrent_base_url,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_username",
        "用户名",
        &settings.qbittorrent_username,
        false,
    );
    settings_form::add_secret_control(
        state,
        &form,
        &qbit,
        "qbittorrent_password",
        "密码",
        settings.qbittorrent_password_configured,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_save_path",
        "保存路径",
        &settings.qbittorrent_save_path,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_category",
        "分类",
        &settings.qbittorrent_category,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_tags",
        "标签",
        &settings.qbittorrent_tags,
        false,
    );
    let test = icon_button("network-wired-symbolic", "测试 qBittorrent 连接");
    let test_row = adw::ActionRow::new();
    test_row.set_title("连接测试");
    test_row.add_suffix(&test);
    qbit.add(&test_row);
    test.connect_clicked({
        let state = state.clone();
        move |_| settings_actions::test_qbittorrent(&state)
    });
    preferences.add(&qbit);
}
