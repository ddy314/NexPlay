use super::super::components::*;
use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::{settings_actions, settings_form};

pub(crate) fn append_bangumi_group(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    preferences: &adw::PreferencesPage,
    settings: &FrontendEditableSettings,
) {
    let bangumi = adw::PreferencesGroup::new();
    bangumi.set_title("Bangumi 账户与元数据");
    settings_form::add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_enabled",
        "启用 Bangumi",
        "用于搜索、发现和同步",
        settings.bangumi_enabled,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_base_url",
        "API 地址",
        &settings.bangumi_base_url,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_oauth_base_url",
        "OAuth 地址",
        &settings.bangumi_oauth_base_url,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_client_id",
        "OAuth Client ID",
        &settings.bangumi_client_id,
        false,
    );
    settings_form::add_secret_control(
        state,
        &form,
        &bangumi,
        "bangumi_client_secret",
        "OAuth Client Secret",
        settings.bangumi_client_secret_configured,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_redirect_uri",
        "Loopback 回调地址",
        &settings.bangumi_redirect_uri,
        false,
    );
    settings_form::add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_user_agent",
        "User-Agent",
        &settings.bangumi_user_agent,
        false,
    );
    settings_form::add_spin_control(
        state,
        &form,
        &bangumi,
        "bangumi_timeout",
        "请求超时（秒）",
        1.0,
        300.0,
        1.0,
        settings.bangumi_request_timeout_secs as f64,
    );
    settings_form::add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_auto_match",
        "自动匹配",
        "扫描后尝试匹配元数据",
        settings.bangumi_auto_match,
    );
    settings_form::add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_cache_images",
        "缓存图片",
        "保存封面，减少重复加载",
        settings.bangumi_cache_images,
    );
    settings_form::add_secret_control(
        state,
        &form,
        &bangumi,
        "bangumi_access_token",
        "Bangumi Access Token",
        settings.bangumi_access_token_configured,
    );
    let auth_row = action_row();
    auth_row.set_title("Bangumi 账户");
    auth_row.set_subtitle(if settings.bangumi_access_token_configured {
        "已连接，可同步收藏和观看状态"
    } else {
        "未连接，发现和同步功能不可用"
    });
    let login = icon_button(
        "contact-new-symbolic",
        if settings.bangumi_access_token_configured {
            "重新连接 Bangumi"
        } else {
            "连接 Bangumi"
        },
    );
    auth_row.add_suffix(&login);
    if settings.bangumi_access_token_configured {
        let logout = icon_button("system-log-out-symbolic", "断开 Bangumi");
        auth_row.add_suffix(&logout);
        let state_for_logout = state.clone();
        logout
            .connect_clicked(move |_| settings_actions::logout_bangumi_account(&state_for_logout));
    }
    bangumi.add(&auth_row);
    {
        let state = state.clone();
        login.connect_clicked(move |_| settings_actions::start_bangumi_oauth(&state));
    }
    let sync_row = action_row();
    sync_row.set_title("同步 Bangumi");
    sync_row.set_subtitle("更新收藏和观看状态");
    let sync = icon_button("emblem-synchronizing-symbolic", "立即同步");
    sync_row.add_suffix(&sync);
    sync.connect_clicked({
        let state = state.clone();
        move |_| settings_actions::sync_bangumi_account(&state)
    });
    bangumi.add(&sync_row);
    preferences.add(&bangumi);
}
