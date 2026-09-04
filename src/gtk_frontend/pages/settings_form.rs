use super::super::components::*;
use super::super::prelude::*;
use super::super::state::{SettingsForm, UiState};
use super::settings_actions;

pub(crate) fn render_media_group(state: &Rc<UiState>, form: &Rc<SettingsForm>) {
    let anchor = form
        .media_group
        .first_child()
        .and_then(|child| child.next_sibling())
        .or_else(|| form.media_group.first_child());
    if let Some(anchor) = anchor {
        let mut next = anchor.next_sibling();
        while let Some(child) = next {
            next = child.next_sibling();
            form.media_group.remove(&child);
        }
    }
    let paths = form.media_libraries.borrow().clone();
    if paths.is_empty() {
        let row = action_row();
        row.set_title("尚未配置媒体目录");
        row.set_subtitle("添加一个目录后，媒体库就可以开始扫描");
        form.media_group.add(&row);
    } else {
        for (index, path) in paths.into_iter().enumerate() {
            let row = action_row();
            row.set_title(&path);
            row.set_subtitle("本地媒体来源");
            let remove = icon_button("list-remove-symbolic", "移除此目录");
            row.add_suffix(&remove);
            let state = state.clone();
            let form_for_callback = form.clone();
            remove.connect_clicked(move |_| {
                if index < form_for_callback.media_libraries.borrow().len() {
                    form_for_callback.media_libraries.borrow_mut().remove(index);
                    settings_changed(&state);
                    render_media_group(&state, &form_for_callback);
                }
            });
            form.media_group.add(&row);
        }
    }
}

pub(crate) fn settings_changed(state: &Rc<UiState>) {
    state.settings_dirty.set(true);
    let generation = state.settings_save_generation.get().saturating_add(1);
    state.settings_save_generation.set(generation);
    schedule_settings_save(state, generation);
}

pub(crate) fn schedule_settings_save(state: &Rc<UiState>, generation: u64) {
    let weak = Rc::downgrade(state);
    glib::timeout_add_local(Duration::from_millis(450), move || {
        let Some(state) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if state.settings_save_generation.get() != generation || !state.settings_dirty.get() {
            return glib::ControlFlow::Break;
        }
        if state.settings_save_in_flight.get() {
            return glib::ControlFlow::Break;
        }
        let Some(form) = state.settings_form.borrow().as_ref().cloned() else {
            return glib::ControlFlow::Break;
        };
        state.settings_save_in_flight.set(true);
        settings_actions::save_settings(&state, &form, generation);
        glib::ControlFlow::Break
    });
}

pub(crate) fn add_secret_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    configured: bool,
) {
    let row = action_row();
    row.set_title(title);
    row.set_subtitle(if configured { "已配置" } else { "未配置" });
    let edit = icon_button("document-edit-symbolic", "修改");
    row.add_suffix(&edit);
    group.add(&row);

    let key = key.to_string();
    let title = title.to_string();
    let state_for_edit = state.clone();
    let form_for_edit = form.clone();
    let row_for_edit = row.clone();
    edit.connect_clicked(move |_| {
        let dialog = adw::Dialog::builder()
            .title(&title)
            .content_width(560)
            .content_height(220)
            .build();
        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&adw::WindowTitle::new(&title, "")));
        let cancel = icon_button("window-close-symbolic", "取消");
        let dialog_for_cancel = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog_for_cancel.close();
        });
        header.pack_start(&cancel);
        let apply = icon_button("object-select-symbolic", "应用");
        header.pack_end(&apply);
        toolbar.add_top_bar(&header);

        let entry = adw::PasswordEntryRow::builder()
            .title("输入新的值")
            .text(
                form_for_edit
                    .secret_values
                    .borrow()
                    .get(&key)
                    .cloned()
                    .unwrap_or_default(),
            )
            .build();
        let page = adw::PreferencesPage::new();
        let entry_group = adw::PreferencesGroup::new();
        entry_group.add(&entry);
        page.add(&entry_group);
        toolbar.set_content(Some(&page));
        dialog.set_child(Some(&toolbar));

        let state_for_apply = state_for_edit.clone();
        let form_for_apply = form_for_edit.clone();
        let row_for_apply = row_for_edit.clone();
        let key_for_apply = key.clone();
        let dialog_for_apply = dialog.clone();
        apply.connect_clicked(move |_| {
            let value = entry.text().to_string();
            let is_configured = !value.trim().is_empty() || configured;
            form_for_apply
                .secret_values
                .borrow_mut()
                .insert(key_for_apply.clone(), value);
            row_for_apply.set_subtitle(if is_configured {
                "已配置"
            } else {
                "未配置"
            });
            settings_changed(&state_for_apply);
            dialog_for_apply.close();
        });
        dialog.present(Some(&state_for_edit.window));
    });
}

pub(crate) fn install_settings_focus_behavior(preferences: &adw::PreferencesPage) {
    preferences.set_focusable(true);
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let preferences_for_click = preferences.clone();
    click.connect_pressed(move |_, _, x, y| {
        let is_entry = preferences_for_click
            .pick(x, y, gtk::PickFlags::DEFAULT)
            .is_some_and(|widget| {
                widget.is::<adw::EntryRow>()
                    || widget.ancestor(adw::EntryRow::static_type()).is_some()
            });
        if !is_entry {
            preferences_for_click.grab_focus();
        }
    });
    preferences.add_controller(click);
}

pub(crate) fn add_entry_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    value: &str,
    password: bool,
) {
    let weak = Rc::downgrade(state);
    if password {
        let row = adw::PasswordEntryRow::builder()
            .title(title)
            .text(value)
            .build();
        row.connect_changed(move |_| {
            if let Some(state) = weak.upgrade() {
                settings_changed(&state);
            }
        });
        group.add(&row);
        form.controls
            .borrow_mut()
            .insert(key.to_string(), row.upcast());
    } else {
        let row = adw::EntryRow::builder().title(title).text(value).build();
        row.connect_changed(move |_| {
            if let Some(state) = weak.upgrade() {
                settings_changed(&state);
            }
        });
        group.add(&row);
        form.controls
            .borrow_mut()
            .insert(key.to_string(), row.upcast());
    }
}

pub(crate) fn add_switch_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    subtitle: &str,
    value: bool,
) {
    let row = adw::SwitchRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.set_active(value);
    let weak = Rc::downgrade(state);
    row.connect_active_notify(move |_| {
        if let Some(state) = weak.upgrade() {
            settings_changed(&state);
        }
    });
    group.add(&row);
    form.controls
        .borrow_mut()
        .insert(key.to_string(), row.upcast());
}

pub(crate) fn add_spin_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
) {
    let row = adw::SpinRow::with_range(min, max, step);
    row.set_title(title);
    row.set_value(value);
    let weak = Rc::downgrade(state);
    row.connect_value_notify(move |_| {
        if let Some(state) = weak.upgrade() {
            settings_changed(&state);
        }
    });
    group.add(&row);
    form.controls
        .borrow_mut()
        .insert(key.to_string(), row.upcast());
}

pub(crate) fn add_combo_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    values: &[&str],
    selected: &str,
) {
    let model = gtk::StringList::new(values);
    let row = adw::ComboRow::builder().title(title).model(&model).build();
    let selected = values
        .iter()
        .position(|value| *value == selected)
        .unwrap_or(0) as u32;
    row.set_selected(selected);
    let weak = Rc::downgrade(state);
    row.connect_selected_notify(move |_| {
        if let Some(state) = weak.upgrade() {
            settings_changed(&state);
            settings_actions::apply_theme_from_form(&state);
        }
    });
    group.add(&row);
    form.controls
        .borrow_mut()
        .insert(key.to_string(), row.upcast());
}

pub(crate) fn control_text(form: &SettingsForm, key: &str) -> String {
    let Some(widget) = form.controls.borrow().get(key).cloned() else {
        return String::new();
    };
    if let Ok(row) = widget.clone().downcast::<adw::PasswordEntryRow>() {
        return row.text().to_string();
    }
    widget
        .downcast::<adw::EntryRow>()
        .map(|row| row.text().to_string())
        .unwrap_or_default()
}

pub(crate) fn control_switch(form: &SettingsForm, key: &str) -> bool {
    form.controls
        .borrow()
        .get(key)
        .and_then(|widget| {
            widget
                .downcast_ref::<adw::SwitchRow>()
                .map(|row| row.is_active())
        })
        .unwrap_or(false)
}

pub(crate) fn control_spin(form: &SettingsForm, key: &str) -> u64 {
    form.controls
        .borrow()
        .get(key)
        .and_then(|widget| {
            widget
                .downcast_ref::<adw::SpinRow>()
                .map(|row| row.value().round() as u64)
        })
        .unwrap_or(1)
}

pub(crate) fn control_combo(form: &SettingsForm, key: &str) -> String {
    form.controls
        .borrow()
        .get(key)
        .and_then(|widget| widget.downcast_ref::<adw::ComboRow>())
        .and_then(|row| row.selected_item())
        .and_then(|object| object.downcast::<gtk::StringObject>().ok())
        .map(|object| object.string().to_string())
        .unwrap_or_default()
}

pub(crate) fn secret_value(form: &SettingsForm, key: &str) -> String {
    form.secret_values
        .borrow()
        .get(key)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn settings_input(form: &SettingsForm) -> FrontendEditableSettings {
    let mut input = form.base.clone();
    input.media_libraries = form.media_libraries.borrow().clone();
    input.bangumi_enabled = control_switch(form, "bangumi_enabled");
    input.bangumi_base_url = control_text(form, "bangumi_base_url");
    input.bangumi_oauth_base_url = control_text(form, "bangumi_oauth_base_url");
    input.bangumi_client_id = control_text(form, "bangumi_client_id");
    input.bangumi_client_secret = secret_value(form, "bangumi_client_secret");
    input.bangumi_redirect_uri = control_text(form, "bangumi_redirect_uri");
    input.bangumi_access_token = secret_value(form, "bangumi_access_token");
    input.bangumi_user_agent = control_text(form, "bangumi_user_agent");
    input.bangumi_request_timeout_secs = control_spin(form, "bangumi_timeout");
    input.bangumi_auto_match = control_switch(form, "bangumi_auto_match");
    input.bangumi_cache_images = control_switch(form, "bangumi_cache_images");
    input.dandanplay_app_id = control_text(form, "dandanplay_app_id");
    input.dandanplay_app_secret = secret_value(form, "dandanplay_app_secret");
    input.dandanplay_api_key = secret_value(form, "dandanplay_api_key");
    input.nyaa_enabled = control_switch(form, "nyaa_enabled");
    input.nyaa_base_url = control_text(form, "nyaa_base_url");
    input.nyaa_category = control_text(form, "nyaa_category");
    input.qbittorrent_enabled = control_switch(form, "qbittorrent_enabled");
    input.qbittorrent_base_url = control_text(form, "qbittorrent_base_url");
    input.qbittorrent_username = control_text(form, "qbittorrent_username");
    input.qbittorrent_password = secret_value(form, "qbittorrent_password");
    input.qbittorrent_save_path = control_text(form, "qbittorrent_save_path");
    input.qbittorrent_category = control_text(form, "qbittorrent_category");
    input.qbittorrent_tags = control_text(form, "qbittorrent_tags");
    input.theme = control_combo(form, "theme");
    input.reduced_motion = control_switch(form, "reduced_motion");
    input.analytics_enabled = control_switch(form, "analytics_enabled");
    input.daily_minutes_goal = control_spin(form, "daily_minutes");
    input.weekly_episodes_goal = control_spin(form, "weekly_episodes");
    input.weekly_active_days_goal = control_spin(form, "weekly_active_days");
    input.logging_level = control_combo(form, "logging_level");
    input
}
