use super::super::prelude::*;
use super::super::{
    events, oauth,
    state::{SettingsForm, UiState},
};
use super::{settings, settings_form};

pub(crate) fn save_settings(state: &Rc<UiState>, form: &Rc<SettingsForm>, generation: u64) {
    let input = settings_form::settings_input(form);
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| save_settings_config(context, input),
        move |result: Result<FrontendEditableSettings, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.settings_save_in_flight.set(false);
            match result {
                Ok(settings) => {
                    state.settings_data.replace(Some(settings));
                    if state.settings_save_generation.get() == generation {
                        apply_theme_from_settings(&state);
                    }
                    events::request_snapshot(&state);
                    if state.settings_save_generation.get() == generation {
                        state.settings_dirty.set(false);
                    } else if state.settings_dirty.get() {
                        let next_generation = state.settings_save_generation.get();
                        settings_form::schedule_settings_save(&state, next_generation);
                    }
                }
                Err(error) => {
                    events::show_error(&state, format!("保存设置失败：{error}"));
                }
            }
        },
    );
}

pub(crate) fn apply_theme_from_form(state: &Rc<UiState>) {
    let Some(form) = state.settings_form.borrow().as_ref().cloned() else {
        return;
    };
    let theme = settings_form::control_combo(&form, "theme");
    let reduced_motion = settings_form::control_switch(&form, "reduced_motion");
    apply_runtime_settings(&theme, reduced_motion);
}

pub(crate) fn apply_theme_from_settings(state: &Rc<UiState>) {
    if let Some(settings) = state.settings_data.borrow().as_ref() {
        apply_runtime_settings(&settings.theme, settings.reduced_motion);
    }
}

pub(crate) fn apply_runtime_settings(theme: &str, reduced_motion: bool) {
    apply_theme(theme);
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_enable_animations(!reduced_motion);
    }
}

pub(crate) fn apply_theme(theme: &str) {
    let scheme = match theme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

pub(crate) fn start_bangumi_oauth(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| start_bangumi_login(context),
        move |result: Result<BangumiLoginStartData, String>| {
            let Some(state) = weak.upgrade() else { return };
            let Ok(start) = result else {
                events::show_error(
                    &state,
                    result
                        .err()
                        .unwrap_or_else(|| "Bangumi 登录启动失败".to_string()),
                );
                return;
            };
            let receiver = match oauth::bind_loopback(&start.redirect_uri, &start.state) {
                Ok(receiver) => receiver,
                Err(error) => {
                    events::show_error(&state, error);
                    return;
                }
            };
            if let Err(error) = oauth::open_default_browser(&start.authorize_url) {
                events::show_error(&state, format!("无法打开默认浏览器：{error}"));
                return;
            }
            events::show_success(&state, "已打开 Bangumi 授权页面");
            let runtime = state.runtime.clone();
            let weak = Rc::downgrade(&state);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                match receiver.try_recv() {
                    Ok(Ok(callback)) => {
                        let weak = weak.clone();
                        runtime.submit(
                            move |context| {
                                complete_bangumi_oauth(
                                    context,
                                    BangumiCompleteOAuthInput {
                                        code: callback.code,
                                        state: callback.state,
                                    },
                                )
                            },
                            move |result: Result<BangumiAuthStatusData, String>| {
                                if let Some(state) = weak.upgrade() {
                                    match result {
                                        Ok(_) => {
                                            state.settings_form.replace(None);
                                            state.settings_data.replace(None);
                                            state.settings_dirty.set(false);
                                            events::show_success(
                                                &state,
                                                "Bangumi 登录完成，正在同步状态",
                                            );
                                            events::request_snapshot(&state);
                                            settings::render_settings(&state);
                                        }
                                        Err(error) => events::show_error(
                                            &state,
                                            format!("Bangumi 登录失败：{error}"),
                                        ),
                                    }
                                }
                            },
                        );
                        glib::ControlFlow::Break
                    }
                    Ok(Err(error)) => {
                        if let Some(state) = weak.upgrade() {
                            events::show_error(&state, error);
                        }
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
                }
            });
        },
    );
}

pub(crate) fn sync_bangumi_account(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        sync_bangumi_now,
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        events::show_success(&state, summary.message);
                        events::request_snapshot(&state);
                    }
                    Err(error) => events::show_error(&state, format!("Bangumi 同步失败：{error}")),
                }
            }
        },
    );
}

pub(crate) fn logout_bangumi_account(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        logout_bangumi,
        move |result: Result<BangumiAuthStatusData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(_) => {
                        state.settings_form.replace(None);
                        state.settings_data.replace(None);
                        state.settings_dirty.set(false);
                        events::show_success(&state, "已退出 Bangumi");
                        events::request_snapshot(&state);
                        settings::render_settings(&state);
                    }
                    Err(error) => events::show_error(&state, format!("退出 Bangumi 失败：{error}")),
                }
            }
        },
    );
}

pub(crate) fn test_qbittorrent(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| Ok(test_qbittorrent_connection(context)),
        move |result: Result<crate::backend_api::ConnectionTestResponse, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(response) if response.ok => events::show_success(&state, response.message),
                    Ok(response) => events::show_error(&state, response.message),
                    Err(error) => {
                        events::show_error(&state, format!("qBittorrent 测试失败：{error}"))
                    }
                }
            }
        },
    );
}
