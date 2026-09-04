use super::prelude::*;
use super::{
    components, events,
    images::ImageLoader,
    pages,
    runtime::BackendRuntime,
    state::{self, UiState},
};

pub(crate) fn build_main_ui(context: AppContext, window: &adw::ApplicationWindow) -> gtk::Widget {
    let image_cache_dir = ImageLoader::cache_root(&context.media.config_snapshot().database.path);
    let runtime = Rc::new(BackendRuntime::new(context));
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    let (home_page, home) = components::page_surface();
    let (discover_page, discover) = components::page_surface();
    let (library_page, library) = components::page_surface();
    let (search_page, search) = components::page_surface();
    let (downloads_page, downloads) = components::page_surface();
    let (insights_page, insights) = components::page_surface();
    let (settings_page, settings) = components::page_surface();
    stack.add_titled_with_icon(&home_page, Some("home"), "首页", "go-home-symbolic");
    stack.add_titled_with_icon(&discover_page, Some("discover"), "发现", "compass-symbolic");
    stack.add_titled_with_icon(
        &library_page,
        Some("library"),
        "媒体库",
        "folder-videos-symbolic",
    );
    stack.add_titled_with_icon(
        &search_page,
        Some("search"),
        "搜索",
        "system-search-symbolic",
    );
    stack.add_titled_with_icon(
        &downloads_page,
        Some("downloads"),
        "下载",
        "folder-download-symbolic",
    );
    stack.add_titled_with_icon(
        &insights_page,
        Some("insights"),
        "洞察",
        "view-statistics-symbolic",
    );
    stack.add_titled_with_icon(
        &settings_page,
        Some("settings"),
        "设置",
        "emblem-system-symbolic",
    );

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("搜索本地或 Bangumi 条目"));
    let toast = adw::ToastOverlay::new();
    toast.set_child(Some(&stack));
    let navigation = adw::NavigationView::new();
    let main_toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let page_title = adw::WindowTitle::new("首页", "");
    header.set_title_widget(Some(&page_title));
    let search_button = gtk::Button::from_icon_name("system-search-symbolic");
    search_button.set_tooltip_text(Some("搜索条目"));
    header.pack_end(&search_button);
    main_toolbar.add_top_bar(&header);
    main_toolbar.set_content(Some(&toast));
    let root_page = adw::NavigationPage::with_tag(&main_toolbar, "首页", "root");
    navigation.add(&root_page);

    let sidebar_toolbar = adw::ToolbarView::new();
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.set_title_widget(Some(&adw::WindowTitle::new("NexPlay", "媒体中心")));
    let primary_menu = gtk::MenuButton::new();
    primary_menu.set_icon_name("open-menu-symbolic");
    primary_menu.set_tooltip_text(Some("主菜单"));
    sidebar_header.pack_start(&primary_menu);
    sidebar_toolbar.add_top_bar(&sidebar_header);
    let switcher = adw::ViewSwitcherSidebar::new();
    switcher.set_stack(Some(&stack));
    sidebar_toolbar.set_content(Some(&switcher));
    let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "导航");
    let content_page = adw::NavigationPage::new(&navigation, "内容");
    let split = adw::NavigationSplitView::new();
    split.set_sidebar(Some(&sidebar_page));
    split.set_content(Some(&content_page));
    split.set_sidebar_width_fraction(0.22);
    split.set_show_content(true);
    let compact = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 860px")
            .expect("valid compact window breakpoint"),
    );
    compact.add_setter(&split, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(compact);

    let empty = state::empty_snapshot();
    let state = Rc::new(UiState {
        runtime,
        window: window.clone(),
        stack: stack.clone(),
        navigation,
        toast,
        images: ImageLoader::new(image_cache_dir),
        snapshot: RefCell::new(empty),
        home,
        discover,
        library,
        search,
        downloads,
        insights,
        settings,
        search_entry: search_entry.clone(),
        search_list: RefCell::new(None),
        search_results: RefCell::new(Vec::new()),
        search_online: RefCell::new(Vec::new()),
        search_error: RefCell::new(None),
        search_generation: Cell::new(0),
        home_feed: RefCell::new(None),
        home_feed_requested: Cell::new(false),
        home_feed_error: RefCell::new(None),
        discovery_feed: RefCell::new(None),
        discovery_requested: Cell::new(false),
        discovery_feed_error: RefCell::new(None),
        downloads_data: RefCell::new(None),
        downloads_requested: Cell::new(false),
        downloads_error: RefCell::new(None),
        insights_data: RefCell::new(None),
        insights_requested: Cell::new(false),
        insights_error: RefCell::new(None),
        insight_range: Cell::new(InsightRange::Week),
        library_grid: Cell::new(true),
        library_cloud: Cell::new(false),
        library_sort: Cell::new(0),
        library_render_generation: Cell::new(0),
        logs: RefCell::new(Vec::new()),
        scan_message: RefCell::new(String::new()),
        scan_fraction: Cell::new(0.0),
        sync_message: RefCell::new(String::new()),
        sync_fraction: Cell::new(0.0),
        sync_loading: Cell::new(false),
        snapshot_loading: Cell::new(false),
        scan_loading: Cell::new(false),
        settings_dirty: Cell::new(false),
        settings_save_generation: Cell::new(0),
        settings_save_in_flight: Cell::new(false),
        settings_form: RefCell::new(None),
        settings_data: RefCell::new(None),
        settings_requested: Cell::new(false),
        settings_error: RefCell::new(None),
        detail_dynamic_refreshes: RefCell::new(HashMap::new()),
        detail_dynamic_in_flight: RefCell::new(HashSet::new()),
        next_page_id: Cell::new(1),
    });

    setup_primary_menu(&state, &primary_menu);

    search_button.connect_clicked({
        let state = state.clone();
        move |_| {
            state.stack.set_visible_child_name("search");
            state.search_entry.grab_focus();
        }
    });
    state.search_entry.connect_search_changed({
        let state = state.clone();
        move |entry| pages::search::search_changed(&state, entry.text().to_string())
    });
    state.search_entry.connect_activate({
        let state = state.clone();
        move |_| {
            if let Some(row) = state
                .search_list
                .borrow()
                .as_ref()
                .and_then(|list| list.selected_row())
            {
                if let Some(subject) = state
                    .search_results
                    .borrow()
                    .get(row.index() as usize)
                    .cloned()
                {
                    pages::detail::open_subject(&state, subject);
                }
            }
        }
    });
    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed({
        let state = state.clone();
        move |_, key, _, _| match key {
            gtk::gdk::Key::Escape => {
                state.search_entry.set_text("");
                state.stack.set_visible_child_name("home");
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Down => {
                if let Some(list) = state.search_list.borrow().as_ref() {
                    list.emit_move_cursor(gtk::MovementStep::VisualPositions, 1, false, false);
                }
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Up => {
                if let Some(list) = state.search_list.borrow().as_ref() {
                    list.emit_move_cursor(gtk::MovementStep::VisualPositions, -1, false, false);
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    state.search_entry.add_controller(key_controller);

    stack.connect_visible_child_name_notify({
        let split = split.clone();
        let page_title = page_title.clone();
        move |stack| {
            split.set_show_content(true);
            let title = match stack.visible_child_name().as_deref() {
                Some("discover") => "发现",
                Some("library") => "媒体库",
                Some("search") => "搜索",
                Some("downloads") => "下载",
                Some("insights") => "洞察",
                Some("settings") => "设置",
                _ => "首页",
            };
            page_title.set_title(title);
        }
    });

    let periodic = Rc::downgrade(&state);
    glib::timeout_add_local(Duration::from_millis(120), move || {
        let Some(state) = periodic.upgrade() else {
            return glib::ControlFlow::Break;
        };
        state.runtime.poll();
        let events = state.runtime.drain_events();
        let mut effects = events::EventEffects::default();
        for event in events {
            effects.merge(events::handle_event(&state, event));
        }
        if effects.render_library {
            pages::library::render_library(&state);
        }
        if effects.render_home {
            pages::home::render_home(&state);
        }
        if effects.render_downloads {
            pages::downloads::render_downloads(&state);
        }
        if effects.refresh_snapshot {
            events::request_snapshot(&state);
        }
        glib::ControlFlow::Continue
    });

    let downloads_timer = Rc::downgrade(&state);
    glib::timeout_add_local(Duration::from_secs(15), move || {
        let Some(state) = downloads_timer.upgrade() else {
            return glib::ControlFlow::Break;
        };
        *state.downloads_data.borrow_mut() = None;
        state.downloads_requested.set(false);
        pages::downloads::render_downloads(&state);
        glib::ControlFlow::Continue
    });

    events::request_snapshot(&state);
    split.upcast()
}

pub(crate) fn setup_primary_menu(state: &Rc<UiState>, button: &gtk::MenuButton) {
    let menu = gio::Menu::new();
    menu.append(Some("偏好设置"), Some("app.preferences"));
    menu.append(Some("键盘快捷键"), Some("app.shortcuts"));
    menu.append(Some("帮助"), Some("app.help"));
    menu.append(Some("关于 NexPlay"), Some("app.about"));
    button.set_menu_model(Some(&menu));

    let Some(application) = state.window.application() else {
        return;
    };

    let preferences = gio::SimpleAction::new("preferences", None);
    preferences.connect_activate({
        let state = state.clone();
        move |_, _| state.stack.set_visible_child_name("settings")
    });
    application.add_action(&preferences);

    let shortcuts = gio::SimpleAction::new("shortcuts", None);
    shortcuts.connect_activate({
        let state = state.clone();
        move |_, _| show_shortcuts_dialog(&state)
    });
    application.add_action(&shortcuts);

    let help = gio::SimpleAction::new("help", None);
    help.connect_activate({
        let state = state.clone();
        move |_, _| show_help_dialog(&state)
    });
    application.add_action(&help);

    let about = gio::SimpleAction::new("about", None);
    about.connect_activate({
        let state = state.clone();
        move |_, _| {
            let dialog = adw::AboutDialog::builder()
                .application_name("NexPlay")
                .application_icon("video-x-generic")
                .version(env!("CARGO_PKG_VERSION"))
                .comments("本地媒体库、Bangumi 元数据与播放进度")
                .license_type(gtk::License::MitX11)
                .website("https://github.com/ddy314/NexPlay")
                .build();
            dialog.present(Some(&state.window));
        }
    });
    application.add_action(&about);
}

pub(crate) fn show_help_dialog(state: &Rc<UiState>) {
    let dialog = adw::AlertDialog::new(
        Some("使用 NexPlay"),
        Some(
            "在首页继续观看；在媒体库扫描和管理本地文件；打开作品详情后，点击集数即可播放或查找资源。",
        ),
    );
    dialog.add_response("close", "关闭");
    dialog.set_close_response("close");
    dialog.present(Some(&state.window));
}

pub(crate) fn show_shortcuts_dialog(state: &Rc<UiState>) {
    let dialog = adw::AlertDialog::new(
        Some("键盘快捷键"),
        Some("Escape 返回上一级或关闭搜索。搜索框中按 Enter 打开当前条目，↑/↓ 移动选择。"),
    );
    dialog.add_response("close", "关闭");
    dialog.set_close_response("close");
    dialog.present(Some(&state.window));
}
