use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::settings_form;

pub(crate) fn append_preferences(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    preferences: &adw::PreferencesPage,
    settings: &FrontendEditableSettings,
) {
    let experience = adw::PreferencesGroup::new();
    experience.set_title("外观与辅助功能");
    settings_form::add_combo_control(
        state,
        &form,
        &experience,
        "theme",
        "主题",
        &["system", "light", "dark"],
        &settings.theme,
    );
    settings_form::add_switch_control(
        state,
        &form,
        &experience,
        "reduced_motion",
        "减少动态效果",
        "减少界面动效",
        settings.reduced_motion,
    );
    preferences.add(&experience);

    let privacy = adw::PreferencesGroup::new();
    privacy.set_title("隐私与洞察");
    settings_form::add_switch_control(
        state,
        &form,
        &privacy,
        "analytics_enabled",
        "记录观看洞察",
        "记录观看时长和完成情况，只保存在本机",
        settings.analytics_enabled,
    );
    settings_form::add_spin_control(
        state,
        &form,
        &privacy,
        "daily_minutes",
        "每日目标（分钟）",
        1.0,
        1440.0,
        1.0,
        settings.daily_minutes_goal as f64,
    );
    settings_form::add_spin_control(
        state,
        &form,
        &privacy,
        "weekly_episodes",
        "每周集数目标",
        1.0,
        100.0,
        1.0,
        settings.weekly_episodes_goal as f64,
    );
    settings_form::add_spin_control(
        state,
        &form,
        &privacy,
        "weekly_active_days",
        "每周活跃天数目标",
        1.0,
        7.0,
        1.0,
        settings.weekly_active_days_goal as f64,
    );
    preferences.add(&privacy);

    let advanced = adw::PreferencesGroup::new();
    advanced.set_title("高级");
    settings_form::add_combo_control(
        state,
        &form,
        &advanced,
        "logging_level",
        "日志级别",
        &["error", "warn", "info", "debug"],
        &settings.logging_level,
    );
    preferences.add(&advanced);
}
