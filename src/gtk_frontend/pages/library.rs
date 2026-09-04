use super::super::components::*;
use super::super::prelude::*;
use super::super::{events, state::UiState};
use super::detail::open_subject;

const LIBRARY_CARD_BATCH_SIZE: usize = 12;

pub(crate) fn render_library(state: &Rc<UiState>) {
    let render_generation = state.library_render_generation.get().wrapping_add(1);
    state.library_render_generation.set(render_generation);
    clear_box(&state.library);
    let snapshot = state.snapshot.borrow().clone();
    let subjects = if state.library_cloud.get() {
        snapshot.bangumi_collections.clone()
    } else {
        snapshot.subjects.clone()
    };
    let source_label = if state.library_cloud.get() {
        "云端收藏"
    } else {
        "本地媒体"
    };
    state.library.append(&page_header(
        "媒体库",
        &format!(
            "在这里管理{}、扫描新文件并打开作品详情。当前有 {} 部条目、{} 个本地文件。",
            source_label,
            subjects.len(),
            subjects.iter().map(|subject| subject.files).sum::<usize>()
        ),
    ));
    let controls = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    let scan_button = action_button(
        if state.scan_loading.get() {
            "扫描中…"
        } else {
            "扫描媒体库"
        },
        "view-refresh-symbolic",
    );
    scan_button.set_sensitive(!state.scan_loading.get());
    let grid_button = gtk::ToggleButton::new();
    grid_button.set_child(Some(&gtk::Image::from_icon_name("view-grid-symbolic")));
    grid_button.set_tooltip_text(Some("网格视图"));
    grid_button.set_active(state.library_grid.get());
    let list_button = gtk::ToggleButton::new();
    list_button.set_child(Some(&gtk::Image::from_icon_name("view-list-symbolic")));
    list_button.set_tooltip_text(Some("列表视图"));
    list_button.set_active(!state.library_grid.get());
    let source_switch = library_source_switch(state);
    let sort = gtk::DropDown::from_strings(&["按年份", "按标题", "按评分"]);
    sort.set_selected(state.library_sort.get());
    controls.append(&scan_button);
    controls.append(&source_switch);
    controls.append(&grid_button);
    controls.append(&list_button);
    controls.append(&sort);
    let settings_button = icon_button("emblem-system-symbolic", "管理媒体目录");
    controls.append(&settings_button);
    state.library.append(&controls);
    {
        let state = state.clone();
        scan_button.connect_clicked(move |_| start_scan(&state));
    }
    {
        let state = state.clone();
        grid_button.connect_clicked(move |_| {
            state.library_grid.set(true);
            render_library(&state);
        });
    }
    {
        let state = state.clone();
        list_button.connect_clicked(move |_| {
            state.library_grid.set(false);
            render_library(&state);
        });
    }
    {
        let state = state.clone();
        sort.connect_selected_notify(move |dropdown| {
            state.library_sort.set(dropdown.selected());
            render_library(&state);
        });
    }
    {
        let state = state.clone();
        settings_button.connect_clicked(move |_| state.stack.set_visible_child_name("settings"));
    }
    if state.scan_loading.get() || !state.scan_message.borrow().is_empty() {
        let progress = gtk::ProgressBar::new();
        progress.set_fraction(state.scan_fraction.get());
        progress.set_show_text(true);
        progress.set_text(Some(&state.scan_message.borrow()));
        state.library.append(&progress);
    }

    if subjects.is_empty() {
        state.library.append(&status(
            if state.library_cloud.get() {
                "云端收藏为空"
            } else {
                "媒体库为空"
            },
            if state.library_cloud.get() {
                "登录 Bangumi 并同步云端收藏，或切换回本地媒体。"
            } else {
                "添加一个媒体目录并启动扫描，视频会按作品和集数整理。"
            },
            "folder-videos-symbolic",
        ));
    } else if state.library_grid.get() {
        let wrap = adaptive_wrap();
        state.library.append(&wrap);
        append_log_panel(state, &state.library);
        schedule_subject_cards(
            state,
            &wrap,
            Rc::new(sorted_subjects(subjects, state.library_sort.get())),
            render_generation,
        );
        return;
    } else {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        for subject in sorted_subjects(subjects, state.library_sort.get()) {
            let row = action_row();
            row.set_title(&subject_title(&subject));
            row.set_subtitle(&format!(
                "{} · {}",
                subject_meta(&subject),
                subject.file_summary
            ));
            row.set_activatable(true);
            let state_for_open = state.clone();
            let subject_for_open = subject.clone();
            row.connect_activated(move |_| open_subject(&state_for_open, subject_for_open.clone()));
            list.append(&row);
        }
        state.library.append(&list);
    }
    append_log_panel(state, &state.library);
}

fn schedule_subject_cards(
    state: &Rc<UiState>,
    wrap: &adw::WrapBox,
    subjects: Rc<Vec<FrontendSubject>>,
    render_generation: u64,
) {
    let weak_state = Rc::downgrade(state);
    let wrap = wrap.clone();
    let next_index = Rc::new(Cell::new(0usize));
    glib::idle_add_local(move || {
        let Some(state) = weak_state.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if state.library_render_generation.get() != render_generation {
            return glib::ControlFlow::Break;
        }

        let start = next_index.get();
        let end = start
            .saturating_add(LIBRARY_CARD_BATCH_SIZE)
            .min(subjects.len());
        for subject in subjects[start..end].iter().cloned() {
            wrap.append(&subject_card(&state, subject));
        }
        next_index.set(end);
        if end == subjects.len() {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn library_source_switch(state: &Rc<UiState>) -> adw::ToggleGroup {
    let source = adw::ToggleGroup::new();
    source.set_homogeneous(true);
    source.set_can_shrink(false);
    source.set_size_request(96, 36);
    source.add_css_class("nx-source-switch");

    let local = adw::Toggle::new();
    local.set_name(Some("local"));
    local.set_icon_name(Some("drive-harddisk-solidstate-symbolic"));
    local.set_tooltip("本地媒体");
    source.add(local);

    let cloud = adw::Toggle::new();
    cloud.set_name(Some("cloud"));
    cloud.set_icon_name(Some("weather-clouds-symbolic"));
    cloud.set_tooltip("云端收藏");
    source.add(cloud);
    source.set_active(if state.library_cloud.get() { 1 } else { 0 });

    let state = state.clone();
    source.connect_active_notify(move |source| {
        state.library_cloud.set(source.active() == 1);
        render_library(&state);
    });

    source
}

pub(crate) fn sorted_subjects(
    mut subjects: Vec<FrontendSubject>,
    sort: u32,
) -> Vec<FrontendSubject> {
    match sort {
        1 => subjects.sort_by_key(|subject| subject_title(subject).to_lowercase()),
        2 => subjects.sort_by(|left, right| right.rating.total_cmp(&left.rating)),
        _ => subjects.sort_by(|left, right| {
            right
                .year
                .cmp(&left.year)
                .then_with(|| subject_title(left).cmp(&subject_title(right)))
        }),
    }
    subjects
}

pub(crate) fn start_scan(state: &Rc<UiState>) {
    if state.scan_loading.replace(true) {
        return;
    }
    state.scan_fraction.set(0.0);
    state.scan_message.replace("扫描已排队…".to_string());
    render_library(state);
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| scan(context),
        move |result: Result<ScanResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.scan_loading.set(false);
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
                    render_library(&state);
                }
            }
        },
    );
}

pub(crate) fn append_log_panel(state: &Rc<UiState>, container: &gtk::Box) {
    if state.logs.borrow().is_empty() {
        return;
    }
    let expander = expander_row();
    expander.set_title("后台日志");
    expander.set_subtitle(&format!("{} 条", state.logs.borrow().len()));
    for message in state.logs.borrow().iter().rev().take(30) {
        let row = action_row();
        row.set_title(message);
        expander.add_row(&row);
    }
    container.append(&expander);
}
