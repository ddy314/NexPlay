mod images;
mod oauth;
mod player;
mod runtime;
mod skeleton;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use adw::prelude::*;

use crate::app::AppContext;
use crate::backend_api::{
    BackendEvent, BackendEventType, BackendSnapshot, CatalogSearchRequest,
    ConfirmResourceDownloadRequest, DiscoveryFeedResponse, DownloadTaskActionRequest,
    DownloadTasksResponse, EpisodeResourcesRequest, EpisodeResourcesResponse,
    FrontendEditableSettings, FrontendEpisode, FrontendSubject, FrontendSubjectDynamic,
    HomeFeedResponse, InsightRange, InsightsDashboardRequest, InsightsDashboardResponse,
    OnlineSubjectRequest, PrepareResourceDownloadRequest, PreparedResourceDownloadResponse,
    RefreshSubjectRequest, ResolveSubjectRequest, ScanResponse, SubjectRef, complete_bangumi_oauth,
    confirm_resource_download, discovery_feed, download_tasks, home_feed, hydrate_subject,
    insights_dashboard, logout_bangumi, online_subject_dynamic, resolve_subject,
    save_settings_config, scan, search_catalog, settings_config, snapshot, start_bangumi_login,
    subject_detail_cache_ready, sync_bangumi_now, test_qbittorrent_connection,
};
use crate::config::{AppConfig, ConfigStore};
use crate::error::AppResult;
use crate::service::{
    BangumiAuthStatusData, BangumiCompleteOAuthInput, BangumiLoginStartData,
    BangumiUpdateEpisodeInput,
};

use self::images::ImageLoader;
use self::player::open_player;
use self::runtime::BackendRuntime;

const APP_ID: &str = "dev.nexplay.NexPlay";
const DETAIL_DYNAMIC_REFRESH_TTL: Duration = Duration::from_secs(6 * 60 * 60);

pub fn run() -> AppResult<()> {
    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_activate(|application| {
        skeleton::install_css();
        apply_runtime_settings("system", false);
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .default_width(1280)
            .default_height(820)
            .title("NexPlay")
            .build();
        let loading = adw::StatusPage::new();
        loading.set_icon_name(Some("folder-videos-symbolic"));
        loading.set_title("正在启动 NexPlay");
        loading.set_description(Some("正在打开本地配置和资料库…"));
        window.set_content(Some(&loading));
        window.present();

        let config_path = native_config_path();
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("nexplay-gtk-bootstrap".to_string())
            .spawn(move || {
                let result =
                    ConfigStore::load_or_create_with_default(config_path, native_default_config())
                        .and_then(AppContext::new)
                        .map_err(|error| error.to_string());
                let _ = sender.send(result);
            })
            .expect("failed to start GTK bootstrap thread");

        glib::timeout_add_local(Duration::from_millis(60), move || {
            match receiver.try_recv() {
                Ok(Ok(context)) => {
                    let root = build_main_ui(context, &window);
                    window.set_content(Some(&root));
                    window.present();
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    let failed = adw::StatusPage::new();
                    failed.set_icon_name(Some("dialog-error-symbolic"));
                    failed.set_title("无法启动 NexPlay");
                    failed.set_description(Some(&error));
                    window.set_content(Some(&failed));
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let failed = adw::StatusPage::new();
                    failed.set_icon_name(Some("dialog-error-symbolic"));
                    failed.set_title("启动线程已停止");
                    window.set_content(Some(&failed));
                    glib::ControlFlow::Break
                }
            }
        });
    });

    // `main` consumes the `gtk` subcommand before constructing the
    // GApplication.  Do not pass that subcommand to GApplication: GLib would
    // interpret it as a file argument and emit the "can not open files"
    // warning instead of activating the window.
    let mut application_args = std::env::args_os().collect::<Vec<_>>();
    if application_args.len() > 1 {
        application_args.remove(1);
    }
    let mut filtered_args = Vec::with_capacity(application_args.len());
    let mut skip_next = false;
    for argument in application_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if argument == "--config" {
            skip_next = true;
            continue;
        }
        if argument.to_string_lossy().starts_with("--config=") {
            continue;
        }
        filtered_args.push(argument);
    }
    application.run_with_args_os(&filtered_args);
    Ok(())
}

fn native_config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("NEXPLAY_CONFIG") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    if let Some(path) = gtk_config_argument() {
        return path;
    }
    let xdg = xdg_config_home().join("nexplay").join("config.toml");
    let mut legacy_candidates = vec![
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("config.toml"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml"),
    ];
    legacy_candidates.dedup();
    for legacy in legacy_candidates {
        if legacy.is_file() && (!xdg.is_file() || legacy_config_is_more_complete(&legacy, &xdg)) {
            return legacy;
        }
    }
    xdg
}

fn gtk_config_argument() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(2);
    while let Some(argument) = args.next() {
        if argument == "--config" {
            return args.next().map(PathBuf::from);
        }
        if let Some(path) = argument.to_string_lossy().strip_prefix("--config=") {
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn legacy_config_is_more_complete(legacy: &Path, native: &Path) -> bool {
    config_media_library_count(legacy) > config_media_library_count(native)
}

fn config_media_library_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| raw.parse::<toml::Table>().ok())
        .and_then(|table| {
            table
                .get("media_libraries")
                .and_then(toml::Value::as_array)
                .cloned()
        })
        .map(|libraries| libraries.len())
        .unwrap_or_default()
}

fn native_default_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.database.path = xdg_data_home().join("nexplay").join("nexplay.sqlite3");
    config
}

fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

struct UiState {
    runtime: Rc<BackendRuntime>,
    window: adw::ApplicationWindow,
    stack: adw::ViewStack,
    navigation: adw::NavigationView,
    toast: adw::ToastOverlay,
    images: ImageLoader,
    snapshot: RefCell<BackendSnapshot>,
    home: gtk::Box,
    discover: gtk::Box,
    library: gtk::Box,
    search: gtk::Box,
    downloads: gtk::Box,
    insights: gtk::Box,
    settings: gtk::Box,
    search_entry: gtk::SearchEntry,
    search_list: RefCell<Option<gtk::ListBox>>,
    search_results: RefCell<Vec<FrontendSubject>>,
    search_online: RefCell<Vec<FrontendSubject>>,
    search_error: RefCell<Option<String>>,
    search_generation: Cell<u64>,
    home_feed: RefCell<Option<HomeFeedResponse>>,
    home_feed_requested: Cell<bool>,
    home_feed_error: RefCell<Option<String>>,
    discovery_feed: RefCell<Option<DiscoveryFeedResponse>>,
    discovery_requested: Cell<bool>,
    discovery_feed_error: RefCell<Option<String>>,
    downloads_data: RefCell<Option<DownloadTasksResponse>>,
    downloads_requested: Cell<bool>,
    downloads_error: RefCell<Option<String>>,
    insights_data: RefCell<Option<InsightsDashboardResponse>>,
    insights_requested: Cell<bool>,
    insights_error: RefCell<Option<String>>,
    insight_range: Cell<InsightRange>,
    library_grid: Cell<bool>,
    library_cloud: Cell<bool>,
    library_sort: Cell<u32>,
    logs: RefCell<Vec<String>>,
    scan_message: RefCell<String>,
    scan_fraction: Cell<f64>,
    sync_message: RefCell<String>,
    sync_fraction: Cell<f64>,
    sync_loading: Cell<bool>,
    snapshot_loading: Cell<bool>,
    scan_loading: Cell<bool>,
    settings_dirty: Cell<bool>,
    settings_save_generation: Cell<u64>,
    settings_save_in_flight: Cell<bool>,
    settings_data: RefCell<Option<FrontendEditableSettings>>,
    settings_requested: Cell<bool>,
    settings_error: RefCell<Option<String>>,
    detail_dynamic_refreshes: RefCell<HashMap<String, Instant>>,
    detail_dynamic_in_flight: RefCell<HashSet<String>>,
    settings_form: RefCell<Option<Rc<SettingsForm>>>,
    next_page_id: Cell<u64>,
}

struct SettingsForm {
    base: FrontendEditableSettings,
    media_libraries: Rc<RefCell<Vec<String>>>,
    controls: RefCell<HashMap<String, gtk::Widget>>,
    secret_values: RefCell<HashMap<String, String>>,
    media_group: adw::PreferencesGroup,
}

fn build_main_ui(context: AppContext, window: &adw::ApplicationWindow) -> gtk::Widget {
    let runtime = Rc::new(BackendRuntime::new(context));
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    let (home_page, home) = page_surface();
    let (discover_page, discover) = page_surface();
    let (library_page, library) = page_surface();
    let (search_page, search) = page_surface();
    let (downloads_page, downloads) = page_surface();
    let (insights_page, insights) = page_surface();
    let (settings_page, settings) = page_surface();
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

    let empty = empty_snapshot();
    let state = Rc::new(UiState {
        runtime,
        window: window.clone(),
        stack: stack.clone(),
        navigation,
        toast,
        images: ImageLoader::new(),
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
        move |entry| search_changed(&state, entry.text().to_string())
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
                    open_subject(&state, subject);
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
        let mut effects = EventEffects::default();
        for event in events {
            effects.merge(handle_event(&state, event));
        }
        if effects.render_library {
            render_library(&state);
        }
        if effects.render_home {
            render_home(&state);
        }
        if effects.render_downloads {
            render_downloads(&state);
        }
        if effects.refresh_snapshot {
            request_snapshot(&state);
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
        render_downloads(&state);
        glib::ControlFlow::Continue
    });

    request_snapshot(&state);
    split.upcast()
}

fn setup_primary_menu(state: &Rc<UiState>, button: &gtk::MenuButton) {
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

fn show_help_dialog(state: &Rc<UiState>) {
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

fn show_shortcuts_dialog(state: &Rc<UiState>) {
    let dialog = adw::AlertDialog::new(
        Some("键盘快捷键"),
        Some("Escape 返回上一级或关闭搜索。搜索框中按 Enter 打开当前条目，↑/↓ 移动选择。"),
    );
    dialog.add_response("close", "关闭");
    dialog.set_close_response("close");
    dialog.present(Some(&state.window));
}

fn empty_snapshot() -> BackendSnapshot {
    BackendSnapshot {
        subjects: Vec::new(),
        bangumi_collections: Vec::new(),
        bangumi_auth: BangumiAuthStatusData {
            configured: false,
            authenticated: false,
            username: None,
            nickname: None,
            avatar_url: None,
            client_configured: false,
            redirect_uri: String::new(),
            pending_sync_count: 0,
            last_error: None,
        },
        stats: crate::backend_api::LibraryStats {
            total: 0,
            matched: 0,
            unmatched: 0,
            tentative: 0,
        },
        settings: crate::backend_api::FrontendSettings {
            bangumi_enabled: false,
            bangumi_auto_match: false,
            bangumi_cache_images: false,
            dandanplay_configured: false,
        },
    }
}

fn page_surface() -> (gtk::ScrolledWindow, gtk::Box) {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.set_margin_top(24);
    page.set_margin_bottom(32);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.set_spacing(18);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1120);
    clamp.set_tightening_threshold(760);
    clamp.set_child(Some(&page));
    (scrolled(&clamp), page)
}

fn clear_box(box_widget: &gtk::Box) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }
}

fn request_snapshot(state: &Rc<UiState>) {
    if state.snapshot_loading.replace(true) {
        return;
    }
    render_home(state);
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| snapshot(context),
        move |result: Result<BackendSnapshot, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.snapshot_loading.set(false);
            match result {
                Ok(snapshot) => {
                    state.snapshot.replace(snapshot);
                    render_all(&state);
                }
                Err(error) => show_error(&state, format!("读取资料库失败：{error}")),
            }
        },
    );
}

fn render_all(state: &Rc<UiState>) {
    render_home(state);
    render_discover(state);
    render_library(state);
    render_search(state);
    render_downloads(state);
    render_insights(state);
    render_settings(state);
}

#[derive(Default)]
struct EventEffects {
    refresh_snapshot: bool,
    render_library: bool,
    render_home: bool,
    render_downloads: bool,
}

impl EventEffects {
    fn merge(&mut self, other: Self) {
        self.refresh_snapshot |= other.refresh_snapshot;
        self.render_library |= other.render_library;
        self.render_home |= other.render_home;
        self.render_downloads |= other.render_downloads;
    }
}

fn handle_event(state: &Rc<UiState>, event: BackendEvent) -> EventEffects {
    if let Some(message) = event.message.as_deref() {
        let mut logs = state.logs.borrow_mut();
        logs.push(message.to_string());
        if logs.len() > 250 {
            let drain = logs.len() - 250;
            logs.drain(0..drain);
        }
        *state.scan_message.borrow_mut() = message.to_string();
    }
    match event.event_type {
        BackendEventType::ScanStarted => {
            state.scan_loading.set(true);
            state.scan_fraction.set(0.0);
            EventEffects {
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::ScanProgress => {
            if let (Some(scanned), Some(indexed)) = (event.scanned, event.indexed) {
                state.scan_fraction.set(if scanned == 0 {
                    0.0
                } else {
                    (indexed as f64 / scanned as f64).clamp(0.0, 1.0)
                });
            }
            EventEffects {
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::ScanFinished | BackendEventType::ScanFailed => {
            state.scan_loading.set(false);
            EventEffects {
                refresh_snapshot: true,
                render_library: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::SubjectUpdated | BackendEventType::ImageCached => EventEffects {
            refresh_snapshot: true,
            ..EventEffects::default()
        },
        BackendEventType::BangumiSyncStarted => {
            state.sync_loading.set(true);
            state.sync_fraction.set(0.0);
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::BangumiSyncProgress => {
            if let (Some(processed), Some(total)) = (event.processed, event.total) {
                state.sync_fraction.set(if total == 0 {
                    0.0
                } else {
                    (processed as f64 / total as f64).clamp(0.0, 1.0)
                });
            }
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::BangumiSyncFinished => {
            state.sync_loading.set(false);
            state.sync_fraction.set(1.0);
            if let Some(message) = event.message {
                state.sync_message.replace(message);
            }
            EventEffects {
                refresh_snapshot: true,
                render_home: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::DownloadCompleted => {
            state.downloads_data.replace(None);
            state.downloads_requested.set(false);
            EventEffects {
                refresh_snapshot: true,
                render_downloads: true,
                ..EventEffects::default()
            }
        }
        BackendEventType::MetadataFailed | BackendEventType::BangumiSyncFailed => {
            if matches!(event.event_type, BackendEventType::BangumiSyncFailed) {
                state.sync_loading.set(false);
            }
            if let Some(message) = event.message {
                show_error(state, message);
            }
            EventEffects {
                refresh_snapshot: true,
                render_home: true,
                ..EventEffects::default()
            }
        }
        _ => EventEffects::default(),
    }
}

fn show_error(state: &Rc<UiState>, message: String) {
    state.toast.add_toast(adw::Toast::new(&message));
}

fn show_success(state: &Rc<UiState>, message: impl Into<String>) {
    state.toast.add_toast(adw::Toast::new(&message.into()));
}

fn label(text: impl AsRef<str>, style: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text.as_ref()));
    label.set_xalign(0.0);
    label.set_wrap(true);
    if !style.is_empty() {
        label.add_css_class(style);
    }
    label
}

fn page_header(title: &str, subtitle: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    header.append(&label(title, "title-1"));
    if !subtitle.is_empty() {
        header.append(&label(subtitle, "dim-label"));
    }
    header
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(child));
    scroll
}

fn status(title: &str, description: &str, icon: &str) -> adw::StatusPage {
    let page = adw::StatusPage::new();
    page.set_icon_name(Some(icon));
    page.set_title(title);
    page.set_description(Some(description));
    page.set_vexpand(true);
    page
}

fn action_button(text: &str, _icon: &str) -> gtk::Button {
    // Ordinary content buttons stay label-only.  Icon-only actions use
    // `icon_button`, which keeps the button hierarchy compact and follows the
    // GNOME button guidance outside a header bar.
    gtk::Button::with_label(text)
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button
}

fn adaptive_wrap() -> adw::WrapBox {
    let wrap = adw::WrapBox::builder()
        .child_spacing(18)
        .line_spacing(18)
        .natural_line_length(1120)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    // The shelf itself should request the width its cards actually need.
    // Giving the wrap box a fill allocation lets its layout distribute the
    // remaining page width, which is especially visible with only two or
    // three items on the home page.
    wrap.set_hexpand(false);
    wrap.set_halign(gtk::Align::Start);
    wrap.set_align(0.0);
    wrap.set_justify_last_line(false);
    wrap
}

fn append_button_row(container: &gtk::Box, title: &str, subtitle: &str, button: &gtk::Button) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    text.append(&label(title, "heading"));
    if !subtitle.is_empty() {
        text.append(&label(subtitle, "dim-label"));
    }
    row.append(&text);
    row.append(button);
    container.append(&row);
}

fn subject_title(subject: &FrontendSubject) -> String {
    if subject.title_cn.trim().is_empty() {
        subject.title.trim().to_string()
    } else {
        subject.title_cn.trim().to_string()
    }
}

fn subject_meta(subject: &FrontendSubject) -> String {
    let mut values = Vec::new();
    if subject.year > 0 {
        values.push(subject.year.to_string());
    }
    if subject.episodes > 0 {
        values.push(format!("{}集", subject.episodes));
    }
    if subject.rating > 0.0 {
        values.push(format!("{:.1}", subject.rating));
    }
    values.join(" · ")
}

fn subject_card(state: &Rc<UiState>, subject: FrontendSubject) -> gtk::Box {
    // The poster is the action surface.  Keep the title and metadata outside
    // the button so a hover never turns unrelated copy into a large card.
    let item = gtk::Box::new(gtk::Orientation::Vertical, 6);
    item.set_width_request(160);
    item.set_hexpand(false);
    item.set_halign(gtk::Align::Start);

    let image = state
        .images
        .widget(&subject.poster, &state.runtime, 160, 226);
    let poster_button = gtk::Button::new();
    poster_button.set_width_request(160);
    poster_button.set_height_request(226);
    poster_button.set_hexpand(false);
    poster_button.set_vexpand(false);
    poster_button.set_halign(gtk::Align::Start);
    poster_button.set_valign(gtk::Align::Start);
    poster_button.set_has_frame(false);
    poster_button.add_css_class("nx-poster-button");

    let poster = gtk::Overlay::new();
    poster.set_width_request(160);
    poster.set_height_request(226);
    poster.set_hexpand(false);
    poster.set_vexpand(false);
    poster.set_halign(gtk::Align::Start);
    poster.set_valign(gtk::Align::Start);
    poster.set_child(Some(&image));

    let hover = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hover.set_hexpand(true);
    hover.set_vexpand(true);
    hover.set_halign(gtk::Align::Fill);
    hover.set_valign(gtk::Align::Fill);
    hover.set_can_target(false);
    hover.add_css_class("nx-poster-hover");
    poster.add_overlay(&hover);

    let play = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play.set_pixel_size(34);
    play.set_halign(gtk::Align::Center);
    play.set_valign(gtk::Align::Center);
    play.set_can_target(false);
    play.add_css_class("nx-poster-play");
    poster.add_overlay(&play);
    poster_button.set_child(Some(&poster));

    let title_text = subject_title(&subject);
    let accessible_name = format!("打开 {title_text}");
    poster_button.update_property(&[gtk::accessible::Property::Label(&accessible_name)]);
    poster_button.set_tooltip_text(Some(&accessible_name));
    item.append(&poster_button);

    let title = label(&title_text, "heading");
    title.set_wrap(false);
    title.set_width_chars(18);
    title.set_max_width_chars(18);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    item.append(&title);
    let meta = label(subject_meta(&subject), "dim-label");
    meta.set_wrap(false);
    meta.set_width_chars(18);
    meta.set_max_width_chars(18);
    meta.set_ellipsize(gtk::pango::EllipsizeMode::End);
    item.append(&meta);

    let state_for_click = state.clone();
    poster_button.connect_clicked(move |_| open_subject(&state_for_click, subject.clone()));
    item
}

fn subject_shelf(
    state: &Rc<UiState>,
    title: &str,
    subtitle: &str,
    subjects: &[FrontendSubject],
) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    section.append(&label(title, "title-2"));
    if !subtitle.is_empty() {
        section.append(&label(subtitle, "dim-label"));
    }
    let wrap = adaptive_wrap();
    for subject in subjects.iter().take(18).cloned() {
        wrap.append(&subject_card(state, subject));
    }
    section.append(&wrap);
    section
}

fn render_home(state: &Rc<UiState>) {
    clear_box(&state.home);
    state.home.append(&page_header(
        "主页",
        "在这里继续观看、回到最近打开的内容，或浏览本地片库。",
    ));
    let actions = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    let discover_button = action_button("打开发现", "compass-symbolic");
    let library_button = action_button("打开媒体库", "folder-videos-symbolic");
    let insights_button = action_button("观看洞察", "view-statistics-symbolic");
    actions.append(&discover_button);
    actions.append(&library_button);
    actions.append(&insights_button);
    state.home.append(&actions);
    {
        let state = state.clone();
        discover_button.connect_clicked(move |_| state.stack.set_visible_child_name("discover"));
    }
    {
        let state = state.clone();
        library_button.connect_clicked(move |_| state.stack.set_visible_child_name("library"));
    }
    {
        let state = state.clone();
        insights_button.connect_clicked(move |_| state.stack.set_visible_child_name("insights"));
    }
    if state.sync_loading.get() {
        let sync_progress = gtk::ProgressBar::new();
        sync_progress.set_fraction(state.sync_fraction.get());
        sync_progress.set_show_text(true);
        sync_progress.set_text(Some(&state.sync_message.borrow()));
        state.home.append(&sync_progress);
    }

    if let Some(feed) = state.home_feed.borrow().clone() {
        let mut has_items = false;
        for section in feed.sections {
            if section.items.is_empty() {
                continue;
            }
            has_items = true;
            let subjects = section
                .items
                .into_iter()
                .map(|item| item.subject)
                .collect::<Vec<_>>();
            state.home.append(&subject_shelf(
                state,
                &section.title,
                &section.subtitle,
                &subjects,
            ));
        }
        if !has_items {
            state.home.append(&status(
                "从第一部番剧开始",
                "在设置中添加媒体目录，然后从媒体库启动扫描。",
                "folder-videos-symbolic",
            ));
        }
    } else {
        if let Some(error) = state.home_feed_error.borrow().clone() {
            let error_page = status(
                "首页内容暂不可用",
                &format!("{error}。可以稍后重试。"),
                "dialog-warning-symbolic",
            );
            let retry = action_button("重试首页内容", "view-refresh-symbolic");
            let state_for_retry = state.clone();
            retry.connect_clicked(move |_| {
                state_for_retry.home_feed_error.replace(None);
                state_for_retry.home_feed_requested.set(false);
                render_home(&state_for_retry);
            });
            error_page.set_child(Some(&retry));
            state.home.append(&error_page);
        } else {
            state.home.append(&skeleton::home());
            if !state.snapshot_loading.get() {
                request_home_feed(state);
            }
        }
    }
}

fn request_home_feed(state: &Rc<UiState>) {
    if state.home_feed_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| home_feed(context),
        move |result: Result<HomeFeedResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.home_feed_requested.set(false);
            match result {
                Ok(feed) => {
                    state.home_feed_error.replace(None);
                    state.home_feed.replace(Some(feed));
                }
                Err(error) => {
                    state.home_feed_error.replace(Some(error));
                }
            }
            render_home(&state);
        },
    );
}

fn render_discover(state: &Rc<UiState>) {
    clear_box(&state.discover);
    state
        .discover
        .append(&page_header("发现", "在这里浏览今日放送和正在上升的作品。"));
    if let Some(feed) = state.discovery_feed.borrow().clone() {
        if feed.today.is_empty() && feed.trending.is_empty() {
            state.discover.append(&status(
                "暂时没有发现内容",
                "公开日历为空或网络暂时不可用，本地媒体库仍然可以正常使用。",
                "compass-symbolic",
            ));
            return;
        }
        if !feed.today.is_empty() {
            state.discover.append(&subject_shelf(
                state,
                "今日放送",
                "Bangumi 每日放送",
                &feed.today,
            ));
        }
        if !feed.trending.is_empty() {
            state.discover.append(&subject_shelf(
                state,
                "正在上升",
                "公开收藏数、评分与排名综合排序",
                &feed.trending,
            ));
        }
    } else if let Some(error) = state.discovery_feed_error.borrow().clone() {
        let error_page = status(
            "发现内容暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试发现", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.discovery_feed_error.replace(None);
            state_for_retry.discovery_requested.set(false);
            render_discover(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.discover.append(&error_page);
    } else {
        state.discover.append(&skeleton::home());
        request_discovery(state);
    }
}

fn request_discovery(state: &Rc<UiState>) {
    if state.discovery_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| discovery_feed(context),
        move |result: Result<DiscoveryFeedResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.discovery_requested.set(false);
            match result {
                Ok(feed) => {
                    state.discovery_feed_error.replace(None);
                    state.discovery_feed.replace(Some(feed));
                }
                Err(error) => {
                    state.discovery_feed_error.replace(Some(error));
                }
            }
            render_discover(&state);
        },
    );
}

fn render_library(state: &Rc<UiState>) {
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
    let local_button = gtk::ToggleButton::new();
    local_button.set_child(Some(&gtk::Image::from_icon_name("folder-videos-symbolic")));
    local_button.set_tooltip_text(Some("本地媒体"));
    local_button.set_active(!state.library_cloud.get());
    let cloud_button = gtk::ToggleButton::new();
    cloud_button.set_child(Some(&gtk::Image::from_icon_name("cloud-symbolic")));
    cloud_button.set_tooltip_text(Some("云端收藏"));
    cloud_button.set_active(state.library_cloud.get());
    local_button.set_group(Some(&cloud_button));
    let sort = gtk::DropDown::from_strings(&["按年份", "按标题", "按评分"]);
    sort.set_selected(state.library_sort.get());
    controls.append(&scan_button);
    controls.append(&local_button);
    controls.append(&cloud_button);
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
        local_button.connect_clicked(move |_| {
            state.library_cloud.set(false);
            render_library(&state);
        });
    }
    {
        let state = state.clone();
        cloud_button.connect_clicked(move |_| {
            state.library_cloud.set(true);
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
        for subject in sorted_subjects(subjects, state.library_sort.get()) {
            wrap.append(&subject_card(state, subject));
        }
        state.library.append(&wrap);
    } else {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        for subject in sorted_subjects(subjects, state.library_sort.get()) {
            let row = adw::ActionRow::new();
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

fn sorted_subjects(mut subjects: Vec<FrontendSubject>, sort: u32) -> Vec<FrontendSubject> {
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

fn start_scan(state: &Rc<UiState>) {
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
                    show_success(&state, "媒体库扫描完成");
                    render_all(&state);
                }
                Err(error) => {
                    show_error(&state, format!("扫描失败：{error}"));
                    render_library(&state);
                }
            }
        },
    );
}

fn append_log_panel(state: &Rc<UiState>, container: &gtk::Box) {
    if state.logs.borrow().is_empty() {
        return;
    }
    let expander = adw::ExpanderRow::new();
    expander.set_title("后台日志");
    expander.set_subtitle(&format!("{} 条", state.logs.borrow().len()));
    for message in state.logs.borrow().iter().rev().take(30) {
        let row = adw::ActionRow::new();
        row.set_title(message);
        expander.add_row(&row);
    }
    container.append(&expander);
}

fn search_changed(state: &Rc<UiState>, query: String) {
    let generation = state.search_generation.get().saturating_add(1);
    state.search_generation.set(generation);
    state.search_online.replace(Vec::new());
    state.search_error.replace(None);
    render_search(state);
    if query.trim().chars().count() < 2 {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| search_catalog(context, CatalogSearchRequest { query, limit: 24 }),
        move |result: Result<crate::backend_api::CatalogSearchResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            if state.search_generation.get() != generation {
                return;
            }
            match result {
                Ok(response) => {
                    state.search_error.replace(None);
                    state.search_online.replace(response.subjects);
                }
                Err(error) => {
                    state.search_error.replace(Some(error));
                }
            }
            render_search(&state);
        },
    );
}

fn render_search(state: &Rc<UiState>) {
    clear_box(&state.search);
    state.search_list.replace(None);
    state.search.append(&page_header(
        "搜索",
        "先过滤本地和云端缓存，输入至少两个字符后在后台延迟查询 Bangumi。Enter 打开选中的条目，Escape 返回。",
    ));
    state.search.append(&state.search_entry);
    let query = state.search_entry.text().to_string().trim().to_lowercase();
    let snapshot = state.snapshot.borrow().clone();
    let mut local = snapshot
        .subjects
        .into_iter()
        .chain(snapshot.bangumi_collections)
        .filter(|subject| subject_matches(subject, &query))
        .collect::<Vec<_>>();
    local = dedupe_subjects(local);
    let online = state.search_online.borrow().clone();
    let results = local.into_iter().chain(online).collect::<Vec<_>>();
    state.search_results.replace(results.clone());
    if query.is_empty() {
        state.search.append(&status(
            "搜索你的资料库",
            "本地结果会即时出现，在线候选只在输入后加载。",
            "system-search-symbolic",
        ));
    } else if results.is_empty() {
        if let Some(error) = state.search_error.borrow().clone() {
            let error_page = status(
                "在线搜索失败",
                &format!("{error}。可以稍后重试。"),
                "dialog-warning-symbolic",
            );
            let retry = action_button("重试搜索", "view-refresh-symbolic");
            let state_for_retry = state.clone();
            let query_for_retry = state.search_entry.text().to_string();
            retry.connect_clicked(move |_| {
                search_changed(&state_for_retry, query_for_retry.clone());
            });
            error_page.set_child(Some(&retry));
            state.search.append(&error_page);
        } else {
            state.search.append(&status(
                "没有匹配条目",
                "可以尝试中文名、日文名、别名或 Bangumi 编号。",
                "system-search-symbolic",
            ));
        }
    } else {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        for subject in results {
            let row = adw::ActionRow::new();
            row.set_title(&subject_title(&subject));
            row.set_subtitle(&subject_meta(&subject));
            row.set_activatable(true);
            let icon = gtk::Image::from_icon_name(if subject.local {
                "folder-videos-symbolic"
            } else {
                "globe-symbolic"
            });
            row.add_prefix(&icon);
            let state_for_row = state.clone();
            let subject_for_row = subject.clone();
            row.connect_activated(move |_| open_subject(&state_for_row, subject_for_row.clone()));
            list.append(&row);
        }
        list.select_row(list.row_at_index(0).as_ref());
        state.search_list.replace(Some(list.clone()));
        state.search.append(&list);
    }
}

fn subject_matches(subject: &FrontendSubject, query: &str) -> bool {
    query.is_empty()
        || subject.title.to_lowercase().contains(query)
        || subject.title_cn.to_lowercase().contains(query)
        || subject
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(query))
        || subject
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn dedupe_subjects(subjects: Vec<FrontendSubject>) -> Vec<FrontendSubject> {
    let mut seen = HashSet::new();
    subjects
        .into_iter()
        .filter(|subject| seen.insert(subject.canonical_key.clone()))
        .collect()
}

fn render_downloads(state: &Rc<UiState>) {
    clear_box(&state.downloads);
    state.downloads.append(&page_header(
        "下载",
        "在这里查看任务进度，暂停、取消或移除下载。",
    ));
    if let Some(data) = state.downloads_data.borrow().clone() {
        if data.tasks.is_empty() {
            state.downloads.append(&status(
                "暂无下载任务",
                "从条目详情打开资源搜索，然后选择 Nyaa 资源加入 qBittorrent。",
                "folder-download-symbolic",
            ));
        } else {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
            list.add_css_class("nx-download-list");
            for task in data.tasks {
                let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
                body.set_hexpand(true);
                body.add_css_class("nx-download-row");
                let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                heading.set_hexpand(true);
                let title = label(&task.title, "heading");
                title.set_hexpand(true);
                title.set_wrap(true);
                heading.append(&title);
                let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                actions.set_valign(gtk::Align::Center);
                if matches!(task.status.as_str(), "paused" | "queued" | "downloading") {
                    let action = if task.status == "paused" {
                        "resume"
                    } else {
                        "pause"
                    };
                    let button = icon_button(
                        if action == "pause" {
                            "media-playback-pause-symbolic"
                        } else {
                            "media-playback-start-symbolic"
                        },
                        if action == "pause" {
                            "暂停"
                        } else {
                            "继续"
                        },
                    );
                    let state_for_action = state.clone();
                    let id = task.id;
                    let action_name = action.to_string();
                    button.connect_clicked(move |_| {
                        control_download(&state_for_action, id, &action_name, false)
                    });
                    actions.append(&button);
                    let cancel = icon_button("process-stop-symbolic", "取消下载");
                    let state_for_cancel = state.clone();
                    let id = task.id;
                    cancel.connect_clicked(move |_| confirm_cancel_download(&state_for_cancel, id));
                    actions.append(&cancel);
                }
                let remove = icon_button("user-trash-symbolic", "删除任务");
                let state_for_remove = state.clone();
                let id = task.id;
                remove.connect_clicked(move |_| confirm_remove_download(&state_for_remove, id));
                actions.append(&remove);
                heading.append(&actions);
                body.append(&heading);
                body.append(&label(
                    &format!(
                        "{} · {} · {} / {} · 速度 {} /s",
                        task.status,
                        if task.stale {
                            "状态过期"
                        } else {
                            "qBittorrent"
                        },
                        format_bytes_i64(task.downloaded),
                        format_bytes_i64(task.size),
                        format_bytes_i64(task.dlspeed),
                    ),
                    "dim-label",
                ));
                let progress_line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let progress = gtk::ProgressBar::new();
                progress.set_fraction(task.progress.clamp(0.0, 1.0));
                progress.set_hexpand(true);
                progress.set_show_text(false);
                progress.set_valign(gtk::Align::Center);
                let percent = label(format!("{:.0}%", task.progress * 100.0), "dim-label");
                percent.set_width_chars(5);
                percent.set_xalign(1.0);
                progress_line.append(&progress);
                progress_line.append(&percent);
                body.append(&progress_line);
                list.append(&body);
            }
            state.downloads.append(&list);
        }
    } else if let Some(error) = state.downloads_error.borrow().clone() {
        let error_page = status(
            "下载状态暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试下载状态", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.downloads_error.replace(None);
            state_for_retry.downloads_requested.set(false);
            render_downloads(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.downloads.append(&error_page);
    } else {
        state.downloads.append(&skeleton::downloads());
        request_downloads(state);
    }
}

fn request_downloads(state: &Rc<UiState>) {
    if state.downloads_requested.replace(true) {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| download_tasks(context),
        move |result: Result<DownloadTasksResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.downloads_requested.set(false);
            match result {
                Ok(data) => {
                    state.downloads_error.replace(None);
                    state.downloads_data.replace(Some(data));
                }
                Err(error) => {
                    state.downloads_error.replace(Some(error));
                }
            };
            render_downloads(&state);
        },
    );
}

fn control_download(state: &Rc<UiState>, task_id: i64, action: &str, delete_files: bool) {
    let weak = Rc::downgrade(state);
    let action = action.to_string();
    state.runtime.submit(
        move |context| {
            crate::backend_api::control_download_task(
                context,
                DownloadTaskActionRequest {
                    task_id,
                    action,
                    delete_files,
                },
            )
        },
        move |result: Result<DownloadTasksResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(data) => {
                    state.downloads_data.replace(Some(data));
                }
                Err(error) => show_error(&state, format!("下载操作失败：{error}")),
            };
            render_downloads(&state);
        },
    );
}

fn confirm_remove_download(state: &Rc<UiState>, task_id: i64) {
    let dialog = adw::AlertDialog::new(
        Some("删除下载任务？"),
        Some("这会删除 NexPlay 的任务记录；是否同时删除 qBittorrent 中的文件由下一步确认。"),
    );
    dialog.add_response("cancel", "取消");
    dialog.add_response("remove", "仅移除记录");
    dialog.add_response("delete", "删除任务和文件");
    dialog.set_default_response(Some("remove"));
    dialog.set_close_response("cancel");
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| match response {
        "remove" => control_download(&state_for_callback, task_id, "remove", false),
        "delete" => control_download(&state_for_callback, task_id, "cancel", true),
        _ => {}
    });
    dialog.present(Some(&state.window));
}

fn confirm_cancel_download(state: &Rc<UiState>, task_id: i64) {
    let dialog = adw::AlertDialog::new(
        Some("取消下载任务？"),
        Some("这会停止 qBittorrent 中的任务并移除 NexPlay 的任务记录，但不会删除已下载文件。"),
    );
    dialog.add_response("keep", "继续下载");
    dialog.add_response("cancel", "取消任务");
    dialog.set_default_response(Some("keep"));
    dialog.set_close_response("keep");
    dialog.set_response_appearance("cancel", adw::ResponseAppearance::Destructive);
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "cancel" {
            control_download(&state_for_callback, task_id, "cancel", false);
        }
    });
    dialog.present(Some(&state.window));
}

fn render_insights(state: &Rc<UiState>) {
    clear_box(&state.insights);
    state.insights.append(&page_header(
        "观看洞察",
        "历史会话和标签统计保留在现有数据库中；GTK 阶段不会产生新的播放会话。",
    ));
    let range = gtk::DropDown::from_strings(&["本周", "本月", "本年"]);
    range.set_selected(match state.insight_range.get() {
        InsightRange::Week => 0,
        InsightRange::Month => 1,
        InsightRange::Year => 2,
    });
    state.insights.append(&range);
    range.connect_selected_notify({
        let state = state.clone();
        move |dropdown| {
            state.insight_range.set(match dropdown.selected() {
                1 => InsightRange::Month,
                2 => InsightRange::Year,
                _ => InsightRange::Week,
            });
            state.insights_data.replace(None);
            state.insights_error.replace(None);
            state.insights_requested.set(false);
            render_insights(&state);
        }
    });
    if let Some(data) = state.insights_data.borrow().clone() {
        let metrics = adw::WrapBox::builder()
            .child_spacing(12)
            .line_spacing(12)
            .line_homogeneous(true)
            .build();
        for (title, value) in [
            ("观看分钟", format!("{:.0}", data.total_minutes)),
            ("完成集数", data.completed_episodes.to_string()),
            ("活跃天数", data.active_days.to_string()),
            ("连续天数", data.streak_days.to_string()),
        ] {
            let group = adw::ActionRow::new();
            group.set_title(title);
            group.set_subtitle(&value);
            group.set_hexpand(true);
            metrics.append(&group);
        }
        state.insights.append(&metrics);
        for ring in data.rings {
            let row = adw::ActionRow::new();
            row.set_title(&ring.label);
            row.set_subtitle(&format!(
                "{:.1} / {:.1} {}",
                ring.value, ring.goal, ring.unit
            ));
            let progress = gtk::ProgressBar::new();
            progress.set_fraction(if ring.goal > 0.0 {
                (ring.value / ring.goal).clamp(0.0, 1.0)
            } else {
                0.0
            });
            progress.set_width_request(180);
            row.add_suffix(&progress);
            state.insights.append(&row);
        }
        if !data.daily.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("每日节奏");
            for point in data.daily.iter().take(14) {
                let row = adw::ActionRow::new();
                row.set_title(&point.label);
                row.set_subtitle(&format!("{:.1}", point.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.dayparts.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("时间分布");
            for point in data.dayparts.iter().take(8) {
                let row = adw::ActionRow::new();
                row.set_title(&point.label);
                row.set_subtitle(&format!("{:.1} 分钟", point.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.tags.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("标签分布");
            for tag in data.tags.iter().take(12) {
                let row = adw::ActionRow::new();
                row.set_title(&tag.label);
                row.set_subtitle(&format!("{:.1}", tag.value));
                group.add(&row);
            }
            state.insights.append(&group);
        }
        if !data.highlights.is_empty() {
            let group = adw::PreferencesGroup::new();
            group.set_title("亮点");
            for highlight in data.highlights.iter().take(8) {
                let row = adw::ActionRow::new();
                row.set_title(&highlight.title);
                row.set_subtitle(&highlight.detail);
                group.add(&row);
            }
            state.insights.append(&group);
        }
        let clear = action_button("清除本地播放历史", "user-trash-symbolic");
        let state_for_clear = state.clone();
        clear.connect_clicked(move |_| confirm_clear_insights(&state_for_clear));
        state.insights.append(&clear);
    } else if let Some(error) = state.insights_error.borrow().clone() {
        let error_page = status(
            "洞察暂不可用",
            &format!("{error}。可以稍后重试。"),
            "dialog-warning-symbolic",
        );
        let retry = action_button("重试洞察", "view-refresh-symbolic");
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.insights_error.replace(None);
            state_for_retry.insights_requested.set(false);
            render_insights(&state_for_retry);
        });
        error_page.set_child(Some(&retry));
        state.insights.append(&error_page);
    } else {
        state.insights.append(&status(
            "正在计算洞察",
            "本地统计会在后台读取现有记录。",
            "view-statistics-symbolic",
        ));
        request_insights(state);
    }
}

fn request_insights(state: &Rc<UiState>) {
    if state.insights_requested.replace(true) {
        return;
    }
    let range = state.insight_range.get();
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| insights_dashboard(context, InsightsDashboardRequest { range }),
        move |result: Result<InsightsDashboardResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            state.insights_requested.set(false);
            match result {
                Ok(data) => {
                    state.insights_error.replace(None);
                    state.insights_data.replace(Some(data));
                }
                Err(error) => {
                    state.insights_error.replace(Some(error));
                }
            }
            render_insights(&state);
        },
    );
}

fn confirm_clear_insights(state: &Rc<UiState>) {
    let dialog = adw::AlertDialog::new(
        Some("清除播放历史？"),
        Some("这只会清除本地播放分析记录，不会删除媒体、Bangumi 状态或下载任务。"),
    );
    dialog.add_response("cancel", "取消");
    dialog.add_response("clear", "清除");
    dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    let state_for_callback = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "clear" {
            return;
        }
        let weak = Rc::downgrade(&state_for_callback);
        state_for_callback.runtime.submit(
            crate::backend_api::clear_playback_analytics,
            move |result: Result<(), String>| {
                if let Some(state) = weak.upgrade() {
                    match result {
                        Ok(()) => {
                            state.insights_data.replace(None);
                            state.insights_requested.set(false);
                            show_success(&state, "播放历史已清除");
                            render_insights(&state);
                        }
                        Err(error) => show_error(&state, format!("清除失败：{error}")),
                    }
                }
            },
        );
    });
    dialog.present(Some(&state.window));
}

fn format_bytes_i64(value: i64) -> String {
    if value < 1024 {
        return format!("{value} B");
    }
    let units = ["KiB", "MiB", "GiB", "TiB"];
    let mut value = value as f64;
    for unit in units {
        value /= 1024.0;
        if value < 1024.0 {
            return format!("{value:.1} {unit}");
        }
    }
    format!("{value:.1} PiB")
}

fn open_subject(state: &Rc<UiState>, subject: FrontendSubject) {
    let detail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    detail.set_vexpand(true);
    detail.set_hexpand(true);
    let tag = format!("subject-{}", state.next_page_id.get());
    state
        .next_page_id
        .set(state.next_page_id.get().saturating_add(1));
    let detail_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let current_subject = Rc::new(RefCell::new(subject.clone()));
    if subject.local && subject.subject_id > 0 {
        let refresh = icon_button("view-refresh-symbolic", "刷新 Bangumi 评分");
        let state_for_refresh = state.clone();
        let current_for_refresh = current_subject.clone();
        let detail_for_refresh = detail.clone();
        refresh.connect_clicked(move |_| {
            refresh_detail(
                &state_for_refresh,
                current_for_refresh.borrow().clone(),
                current_for_refresh.clone(),
                detail_for_refresh.clone(),
            )
        });
        header.pack_end(&refresh);
    }
    if subject.provider == "bangumi" && subject.subject_id > 0 {
        let sync = icon_button("emblem-synchronizing-symbolic", "同步 Bangumi 状态");
        let state_for_sync = state.clone();
        let subject_id = subject.subject_id;
        sync.connect_clicked(move |_| sync_subject(&state_for_sync, subject_id));
        header.pack_end(&sync);
    }
    detail_view.add_top_bar(&header);
    detail_view.set_content(Some(&detail));
    let page = adw::NavigationPage::with_tag(&detail_view, &subject_title(&subject), &tag);
    state.navigation.push(&page);

    // The detail shell is useful even when the remote source is slow or
    // unavailable.  Render the persisted subject first and let the network
    // work below update this same page when it is actually needed.
    render_detail(state, &detail, subject.clone());

    if subject.provider != "bangumi" || subject.provider_subject_id.trim().is_empty() {
        return;
    }

    if subject_detail_cache_ready(&subject) {
        start_detail_dynamic_refresh(state, subject, current_subject, detail, false, false);
    } else {
        start_detail_hydration(state, subject, current_subject, detail, page);
    }
}

enum DetailDynamicRefreshResult {
    Local(FrontendSubject),
    Online(FrontendSubjectDynamic),
}

fn detail_refresh_key(subject: &FrontendSubject) -> String {
    format!("{}:{}", subject.provider, subject.provider_subject_id)
}

fn start_detail_dynamic_refresh(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
    force: bool,
    show_failure: bool,
) -> bool {
    let key = detail_refresh_key(&subject);
    {
        let mut in_flight = state.detail_dynamic_in_flight.borrow_mut();
        if in_flight.contains(&key) {
            return false;
        }
        if !force
            && state
                .detail_dynamic_refreshes
                .borrow()
                .get(&key)
                .is_some_and(|started| started.elapsed() < DETAIL_DYNAMIC_REFRESH_TTL)
        {
            return false;
        }
        in_flight.insert(key.clone());
        state
            .detail_dynamic_refreshes
            .borrow_mut()
            .insert(key.clone(), Instant::now());
    }

    let weak = Rc::downgrade(state);
    let subject_for_request = subject.clone();
    let current_for_result = current_subject.clone();
    let key_for_result = key.clone();
    state.runtime.submit(
        move |context| {
            if subject_for_request.local && subject_for_request.subject_id > 0 {
                crate::backend_api::refresh_subject_metadata(
                    context,
                    RefreshSubjectRequest {
                        subject_id: subject_for_request.subject_id,
                    },
                )
                .map(DetailDynamicRefreshResult::Local)
            } else {
                online_subject_dynamic(
                    context,
                    OnlineSubjectRequest {
                        provider: subject_for_request.provider,
                        provider_subject_id: subject_for_request.provider_subject_id,
                    },
                )
                .map(DetailDynamicRefreshResult::Online)
            }
        },
        move |result: Result<DetailDynamicRefreshResult, String>| {
            let Some(state) = weak.upgrade() else { return };
            state
                .detail_dynamic_in_flight
                .borrow_mut()
                .remove(&key_for_result);
            match result {
                Ok(DetailDynamicRefreshResult::Local(updated)) => {
                    current_for_result.replace(updated.clone());
                    clear_box(&container);
                    render_detail(&state, &container, updated);
                }
                Ok(DetailDynamicRefreshResult::Online(dynamic)) => {
                    let mut updated = current_for_result.borrow().clone();
                    apply_subject_dynamic(&mut updated, dynamic);
                    current_for_result.replace(updated.clone());
                    clear_box(&container);
                    render_detail(&state, &container, updated);
                }
                Err(error) if show_failure => {
                    show_error(&state, format!("刷新 Bangumi 评分失败：{error}"));
                }
                Err(_) => {
                    // Cached content remains the source of truth when an
                    // opportunistic refresh cannot reach Bangumi.
                }
            }
        },
    );
    true
}

fn apply_subject_dynamic(subject: &mut FrontendSubject, dynamic: FrontendSubjectDynamic) {
    if subject.provider == dynamic.provider
        && subject.provider_subject_id == dynamic.provider_subject_id
    {
        if let Some(rating) = dynamic.rating {
            subject.rating = rating;
        }
        if let Some(rank) = dynamic.rank {
            subject.rank = rank;
        }
    }
}

fn start_detail_hydration(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
    page: adw::NavigationPage,
) {
    let subject_ref = SubjectRef {
        canonical_key: subject.canonical_key.clone(),
        provider: subject.provider.clone(),
        provider_subject_id: subject.provider_subject_id.clone(),
        media_id: subject.local_files.first().map(|file| file.media_id),
    };
    let weak = Rc::downgrade(state);
    let page_for_result = page;
    let current_for_result = current_subject;
    let subject_for_request = subject.clone();
    state.runtime.submit(
        move |context| {
            if subject_for_request.local && subject_for_request.subject_id > 0 {
                hydrate_subject(
                    context,
                    RefreshSubjectRequest {
                        subject_id: subject_for_request.subject_id,
                    },
                )
            } else {
                resolve_subject(context, ResolveSubjectRequest { subject_ref })
            }
        },
        move |result: Result<FrontendSubject, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(subject) => {
                    page_for_result.set_title(&subject_title(&subject));
                    current_for_result.replace(subject.clone());
                    clear_box(&container);
                    render_detail(&state, &container, subject);
                }
                Err(error) => {
                    show_error(&state, format!("详情补全失败，已保留当前缓存：{error}"));
                }
            }
        },
    );
}

fn render_detail(state: &Rc<UiState>, container: &gtk::Box, subject: FrontendSubject) {
    let scroll_content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    scroll_content.set_margin_top(18);
    scroll_content.set_margin_bottom(28);
    scroll_content.set_margin_start(28);
    scroll_content.set_margin_end(28);
    let overview = adw::WrapBox::builder()
        .child_spacing(28)
        .line_spacing(24)
        .natural_line_length(920)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    overview.set_hexpand(true);
    overview.set_halign(gtk::Align::Fill);
    overview.set_justify_last_line(false);
    overview.append(&detail_cover(state, &subject));
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 8);
    copy.set_hexpand(true);
    copy.set_width_request(360);
    copy.append(&label(subject_title(&subject), "title-1"));
    if !subject.title.trim().is_empty() && subject.title.trim() != subject_title(&subject) {
        copy.append(&label(&subject.title, "title-3"));
    }
    copy.append(&label(&subject_meta(&subject), "dim-label"));
    let status_text = if subject.local {
        "本地可用"
    } else {
        "在线条目，尚无本地媒体"
    };
    copy.append(&label(status_text, "body"));
    let summary = localized_summary(&subject.summary);
    if !summary.is_empty() {
        copy.append(&label("简介", "heading"));
        let summary_preview = label(summary, "body");
        summary_preview.set_lines(3);
        summary_preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
        summary_preview.set_halign(gtk::Align::Start);
        copy.append(&summary_preview);
        let full_summary = gtk::Button::with_label("查看完整简介");
        full_summary.add_css_class("flat");
        full_summary.set_halign(gtk::Align::Start);
        let state_for_summary = state.clone();
        let title_for_summary = subject_title(&subject);
        let summary_text = summary.to_string();
        full_summary.connect_clicked(move |_| {
            show_summary_dialog(&state_for_summary, &title_for_summary, &summary_text)
        });
        copy.append(&full_summary);
    }
    if !subject.tags.is_empty() {
        copy.append(&label("标签", "heading"));
        let tags = adw::WrapBox::builder()
            .child_spacing(6)
            .line_spacing(6)
            .wrap_policy(adw::WrapPolicy::Minimum)
            .build();
        tags.set_halign(gtk::Align::Start);
        for tag in subject.tags.iter().take(10) {
            let tag_label = label(format!("#{tag}"), "dim-label");
            tag_label.set_wrap(false);
            tag_label.add_css_class("nx-tag");
            tags.append(&tag_label);
        }
        let remaining = subject.tags.len().saturating_sub(10);
        if remaining > 0 {
            tags.append(&label(format!("+{remaining}"), "dim-label"));
        }
        copy.append(&tags);
    }
    copy.append(&label(
        &format!(
            "观看进度：{}/{}（{}%）",
            subject.watched_episodes,
            subject.episodes,
            (subject.progress * 100.0).round()
        ),
        "dim-label",
    ));
    let progress = gtk::ProgressBar::new();
    progress.set_fraction(subject.progress.clamp(0.0, 1.0));
    copy.append(&progress);
    let copy_width = adw::Clamp::new();
    copy_width.set_maximum_size(680);
    copy_width.set_tightening_threshold(400);
    copy_width.set_child(Some(&copy));
    overview.append(&copy_width);
    scroll_content.append(&overview);
    let episodes = gtk::Box::new(gtk::Orientation::Vertical, 8);
    episodes.append(&label(
        format!("集数（{}）", subject.episodes_detail.len()),
        "title-2",
    ));
    episodes.append(&label(
        "点击集数即可播放；没有本地文件的集数会打开资源搜索。",
        "dim-label",
    ));
    let episode_list = build_episode_list(state, &subject);
    episodes.append(&episode_list);
    scroll_content.append(&episodes);
    let content_clamp = adw::Clamp::new();
    content_clamp.set_maximum_size(1200);
    content_clamp.set_tightening_threshold(760);
    content_clamp.set_child(Some(&scroll_content));
    container.append(&scrolled(&content_clamp));
}

fn detail_cover(state: &Rc<UiState>, subject: &FrontendSubject) -> gtk::Widget {
    let poster = state
        .images
        .widget(&subject.poster, &state.runtime, 220, 308);
    let Some(episode) = preferred_playback_episode(subject) else {
        return poster;
    };

    let cover = gtk::Button::new();
    cover.set_has_frame(false);
    cover.set_size_request(220, 308);
    cover.set_hexpand(false);
    cover.set_vexpand(false);
    cover.set_halign(gtk::Align::Start);
    cover.set_valign(gtk::Align::Start);
    cover.set_tooltip_text(Some("播放当前集"));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&poster));
    let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(48);
    play_icon.set_opacity(0.0);
    play_icon.set_halign(gtk::Align::Center);
    play_icon.set_valign(gtk::Align::Center);
    overlay.add_overlay(&play_icon);
    cover.set_child(Some(&overlay));
    let motion = gtk::EventControllerMotion::new();
    let icon_for_enter = play_icon.clone();
    motion.connect_enter(move |_, _, _| icon_for_enter.set_opacity(1.0));
    let icon_for_leave = play_icon.clone();
    motion.connect_leave(move |_| icon_for_leave.set_opacity(0.0));
    cover.add_controller(motion);
    let state_for_play = state.clone();
    let subject_for_play = subject.clone();
    cover.connect_clicked(move |_| {
        open_player(&state_for_play, subject_for_play.clone(), episode.clone())
    });
    cover.upcast()
}

fn show_summary_dialog(state: &Rc<UiState>, title: &str, summary: &str) {
    let dialog = adw::Dialog::builder()
        .title("完整简介")
        .content_width(680)
        .content_height(520)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(title, "完整简介")));
    let close = icon_button("window-close-symbolic", "关闭");
    let dialog_for_close = dialog.clone();
    close.connect_clicked(move |_| {
        dialog_for_close.close();
    });
    header.pack_end(&close);
    toolbar.add_top_bar(&header);
    let text = label(summary, "body");
    text.set_selectable(true);
    text.set_margin_top(24);
    text.set_margin_bottom(28);
    text.set_margin_start(28);
    text.set_margin_end(28);
    toolbar.set_content(Some(&scrolled(&text)));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&state.window));
}

fn localized_summary(summary: &str) -> &str {
    summary
        .split_once("[简介原文]")
        .map(|(localized, _)| localized)
        .unwrap_or(summary)
        .trim()
}

fn episode_title(episode: &crate::backend_api::FrontendEpisode) -> String {
    if episode.title_cn.trim().is_empty() {
        episode.title.trim().to_string()
    } else {
        episode.title_cn.trim().to_string()
    }
}

fn episode_subtitle(episode: &crate::backend_api::FrontendEpisode) -> String {
    let mut states = Vec::new();
    if episode.cached {
        states.push("本地可播放");
    } else {
        states.push("在线");
    }
    states.push(if episode.watched {
        "已观看"
    } else {
        "未观看"
    });
    if !episode.bgm_collection_label.trim().is_empty() {
        states.push(episode.bgm_collection_label.as_str());
    }
    states.join(" · ")
}

fn build_episode_list(state: &Rc<UiState>, subject: &FrontendSubject) -> gtk::ListView {
    let model = gio::ListStore::new::<glib::BoxedAnyObject>();
    for episode in subject.episodes_detail.iter().cloned() {
        model.append(&glib::BoxedAnyObject::new(episode));
    }
    let selection = gtk::NoSelection::new(Some(model));
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = state.clone();
    let subject_for_setup = subject.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        root.set_hexpand(true);
        root.add_css_class("nx-episode-row");
        let content = gtk::Button::new();
        content.set_has_frame(false);
        content.set_hexpand(true);
        content.set_halign(gtk::Align::Fill);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
        text.set_hexpand(true);
        text.append(&label("", "heading"));
        text.append(&label("", "dim-label"));
        content.set_child(Some(&text));
        root.append(&content);
        let watched = icon_button("emblem-ok-symbolic", "标记为已观看");
        watched.set_valign(gtk::Align::Center);
        root.append(&watched);
        list_item.set_child(Some(&root));

        let item_for_content = list_item.clone();
        let state_for_content = state_for_setup.clone();
        let subject_for_content = subject_for_setup.clone();
        content.connect_clicked(move |_| {
            if let Some(episode) = episode_from_list_item(&item_for_content) {
                activate_episode(&state_for_content, subject_for_content.clone(), episode);
            }
        });

        let item_for_watch = list_item.clone();
        let state_for_watch = state_for_setup.clone();
        let subject_id = subject_for_setup.subject_id;
        watched.connect_clicked(move |_| {
            let Some(episode) = episode_from_list_item(&item_for_watch) else {
                return;
            };
            if let Some(episode_id) = episode.bgm_episode_id
                && !episode.watched
                && subject_id > 0
            {
                mark_episode_watched(&state_for_watch, subject_id, episode_id);
            }
        });
    });
    factory.connect_bind(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(episode) = episode_from_list_item(list_item) else {
            return;
        };
        let Some(root) = list_item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(content) = root.first_child().and_downcast::<gtk::Button>() else {
            return;
        };
        let Some(text) = content.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(title) = text.first_child().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(subtitle) = title.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        title.set_text(&format!(
            "第 {} 集 · {}",
            episode.episode,
            episode_title(&episode)
        ));
        subtitle.set_text(&episode_subtitle(&episode));
        if let Some(watched) = root.last_child().and_downcast::<gtk::Button>() {
            watched.set_visible(episode.bgm_episode_id.is_some());
            watched.set_sensitive(episode.bgm_episode_id.is_some() && !episode.watched);
            watched.set_tooltip_text(Some(if episode.watched {
                "已观看"
            } else {
                "标记为已观看"
            }));
        }
    });
    let list = gtk::ListView::new(Some(selection), Some(factory));
    list.set_show_separators(true);
    list.set_vexpand(false);
    list.set_hexpand(true);
    list.add_css_class("nx-episode-list");
    list
}

fn episode_from_list_item(
    list_item: &gtk::ListItem,
) -> Option<crate::backend_api::FrontendEpisode> {
    list_item
        .item()
        .and_downcast::<glib::BoxedAnyObject>()
        .map(|object| {
            object
                .borrow::<crate::backend_api::FrontendEpisode>()
                .clone()
        })
}

fn activate_episode(state: &Rc<UiState>, subject: FrontendSubject, episode: FrontendEpisode) {
    if episode.media_id.is_some() {
        open_player(state, subject, episode);
    } else {
        open_resources(state, subject, episode.episode as f64);
    }
}

fn preferred_playback_episode(
    subject: &FrontendSubject,
) -> Option<crate::backend_api::FrontendEpisode> {
    subject
        .current_episode
        .and_then(|number| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| episode.episode == number && episode.media_id.is_some())
        })
        .or_else(|| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| !episode.watched && episode.media_id.is_some())
        })
        .or_else(|| {
            subject
                .episodes_detail
                .iter()
                .find(|episode| episode.media_id.is_some())
        })
        .cloned()
}

fn refresh_detail(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    current_subject: Rc<RefCell<FrontendSubject>>,
    container: gtk::Box,
) {
    if !subject.local || subject.subject_id <= 0 {
        show_error(state, "在线条目无需刷新本地元数据".to_string());
        return;
    }
    if start_detail_dynamic_refresh(state, subject, current_subject, container, true, true) {
        show_success(state, "已在后台刷新 Bangumi 评分和排名");
    }
}

fn sync_subject(state: &Rc<UiState>, subject_id: i64) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::sync_bangumi_subject(context, RefreshSubjectRequest { subject_id })
        },
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        show_success(&state, summary.message);
                        request_snapshot(&state);
                    }
                    Err(error) => show_error(&state, format!("Bangumi 同步失败：{error}")),
                }
            }
        },
    );
}

fn mark_episode_watched(state: &Rc<UiState>, subject_id: i64, episode_id: i64) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::update_bangumi_episode(
                context,
                BangumiUpdateEpisodeInput {
                    subject_id,
                    episode_id,
                    collection_type: 2,
                },
            )
        },
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        show_success(&state, summary.message);
                        request_snapshot(&state);
                    }
                    Err(error) => show_error(&state, format!("更新观看状态失败：{error}")),
                }
            }
        },
    );
}

fn render_settings(state: &Rc<UiState>) {
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

fn request_settings(state: &Rc<UiState>) {
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
                    apply_runtime_settings(&settings.theme, settings.reduced_motion);
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

fn build_settings_page(state: &Rc<UiState>, settings: FrontendEditableSettings) {
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
                        settings_changed(&state);
                        render_media_group(&state, &form);
                    }
                }
            });
        });
    }
    render_media_group(state, &form);
    preferences.add(&media_group);

    let bangumi = adw::PreferencesGroup::new();
    bangumi.set_title("Bangumi 账户与元数据");
    add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_enabled",
        "启用 Bangumi",
        "用于搜索、发现和同步",
        settings.bangumi_enabled,
    );
    add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_base_url",
        "API 地址",
        &settings.bangumi_base_url,
        false,
    );
    add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_oauth_base_url",
        "OAuth 地址",
        &settings.bangumi_oauth_base_url,
        false,
    );
    add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_client_id",
        "OAuth Client ID",
        &settings.bangumi_client_id,
        false,
    );
    add_secret_control(
        state,
        &form,
        &bangumi,
        "bangumi_client_secret",
        "OAuth Client Secret",
        settings.bangumi_client_secret_configured,
    );
    add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_redirect_uri",
        "Loopback 回调地址",
        &settings.bangumi_redirect_uri,
        false,
    );
    add_entry_control(
        state,
        &form,
        &bangumi,
        "bangumi_user_agent",
        "User-Agent",
        &settings.bangumi_user_agent,
        false,
    );
    add_spin_control(
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
    add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_auto_match",
        "自动匹配",
        "扫描后尝试匹配元数据",
        settings.bangumi_auto_match,
    );
    add_switch_control(
        state,
        &form,
        &bangumi,
        "bangumi_cache_images",
        "缓存图片",
        "保存封面，减少重复加载",
        settings.bangumi_cache_images,
    );
    add_secret_control(
        state,
        &form,
        &bangumi,
        "bangumi_access_token",
        "Bangumi Access Token",
        settings.bangumi_access_token_configured,
    );
    let auth_row = adw::ActionRow::new();
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
        logout.connect_clicked(move |_| logout_bangumi_account(&state_for_logout));
    }
    bangumi.add(&auth_row);
    {
        let state = state.clone();
        login.connect_clicked(move |_| start_bangumi_oauth(&state));
    }
    let sync_row = adw::ActionRow::new();
    sync_row.set_title("同步 Bangumi");
    sync_row.set_subtitle("更新收藏和观看状态");
    let sync = icon_button("emblem-synchronizing-symbolic", "立即同步");
    sync_row.add_suffix(&sync);
    sync.connect_clicked({
        let state = state.clone();
        move |_| sync_bangumi_account(&state)
    });
    bangumi.add(&sync_row);
    preferences.add(&bangumi);

    let dandan = adw::PreferencesGroup::new();
    dandan.set_title("DanDanPlay");
    add_entry_control(
        state,
        &form,
        &dandan,
        "dandanplay_app_id",
        "App ID",
        &settings.dandanplay_app_id,
        false,
    );
    add_secret_control(
        state,
        &form,
        &dandan,
        "dandanplay_app_secret",
        "App Secret",
        settings.dandanplay_app_secret_configured,
    );
    add_secret_control(
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
    add_switch_control(
        state,
        &form,
        &nyaa,
        "nyaa_enabled",
        "启用 Nyaa",
        "详情页资源搜索使用 Nyaa RSS",
        settings.nyaa_enabled,
    );
    add_entry_control(
        state,
        &form,
        &nyaa,
        "nyaa_base_url",
        "服务地址",
        &settings.nyaa_base_url,
        false,
    );
    add_entry_control(
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
    add_switch_control(
        state,
        &form,
        &qbit,
        "qbittorrent_enabled",
        "启用 qBittorrent",
        "连接下载服务以管理资源",
        settings.qbittorrent_enabled,
    );
    add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_base_url",
        "服务地址",
        &settings.qbittorrent_base_url,
        false,
    );
    add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_username",
        "用户名",
        &settings.qbittorrent_username,
        false,
    );
    add_secret_control(
        state,
        &form,
        &qbit,
        "qbittorrent_password",
        "密码",
        settings.qbittorrent_password_configured,
    );
    add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_save_path",
        "保存路径",
        &settings.qbittorrent_save_path,
        false,
    );
    add_entry_control(
        state,
        &form,
        &qbit,
        "qbittorrent_category",
        "分类",
        &settings.qbittorrent_category,
        false,
    );
    add_entry_control(
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
        move |_| test_qbittorrent(&state)
    });
    preferences.add(&qbit);

    let experience = adw::PreferencesGroup::new();
    experience.set_title("外观与辅助功能");
    add_combo_control(
        state,
        &form,
        &experience,
        "theme",
        "主题",
        &["system", "light", "dark"],
        &settings.theme,
    );
    add_switch_control(
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
    add_switch_control(
        state,
        &form,
        &privacy,
        "analytics_enabled",
        "记录观看洞察",
        "记录观看时长和完成情况，只保存在本机",
        settings.analytics_enabled,
    );
    add_spin_control(
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
    add_spin_control(
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
    add_spin_control(
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
    add_combo_control(
        state,
        &form,
        &advanced,
        "logging_level",
        "日志级别",
        &["error", "warn", "info", "debug"],
        &settings.logging_level,
    );
    preferences.add(&advanced);

    install_settings_focus_behavior(&preferences);
    state.settings.append(&preferences);
}

fn render_media_group(state: &Rc<UiState>, form: &Rc<SettingsForm>) {
    if let Some(anchor) = form.media_group.first_child() {
        let mut next = anchor.next_sibling();
        while let Some(child) = next {
            next = child.next_sibling();
            form.media_group.remove(&child);
        }
    }
    let paths = form.media_libraries.borrow().clone();
    if paths.is_empty() {
        let row = adw::ActionRow::new();
        row.set_title("尚未配置媒体目录");
        row.set_subtitle("添加一个目录后，媒体库就可以开始扫描");
        form.media_group.add(&row);
    } else {
        for (index, path) in paths.into_iter().enumerate() {
            let row = adw::ActionRow::new();
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

fn settings_changed(state: &Rc<UiState>) {
    state.settings_dirty.set(true);
    let generation = state.settings_save_generation.get().saturating_add(1);
    state.settings_save_generation.set(generation);
    schedule_settings_save(state, generation);
}

fn schedule_settings_save(state: &Rc<UiState>, generation: u64) {
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
        save_settings(&state, &form, generation);
        glib::ControlFlow::Break
    });
}

fn add_secret_control(
    state: &Rc<UiState>,
    form: &Rc<SettingsForm>,
    group: &adw::PreferencesGroup,
    key: &str,
    title: &str,
    configured: bool,
) {
    let row = adw::ActionRow::new();
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

fn install_settings_focus_behavior(preferences: &adw::PreferencesPage) {
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

fn add_entry_control(
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

fn add_switch_control(
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

fn add_spin_control(
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

fn add_combo_control(
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
            apply_theme_from_form(&state);
        }
    });
    group.add(&row);
    form.controls
        .borrow_mut()
        .insert(key.to_string(), row.upcast());
}

fn control_text(form: &SettingsForm, key: &str) -> String {
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

fn control_switch(form: &SettingsForm, key: &str) -> bool {
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

fn control_spin(form: &SettingsForm, key: &str) -> u64 {
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

fn control_combo(form: &SettingsForm, key: &str) -> String {
    form.controls
        .borrow()
        .get(key)
        .and_then(|widget| widget.downcast_ref::<adw::ComboRow>())
        .and_then(|row| row.selected_item())
        .and_then(|object| object.downcast::<gtk::StringObject>().ok())
        .map(|object| object.string().to_string())
        .unwrap_or_default()
}

fn secret_value(form: &SettingsForm, key: &str) -> String {
    form.secret_values
        .borrow()
        .get(key)
        .cloned()
        .unwrap_or_default()
}

fn settings_input(form: &SettingsForm) -> FrontendEditableSettings {
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

fn save_settings(state: &Rc<UiState>, form: &Rc<SettingsForm>, generation: u64) {
    let input = settings_input(form);
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
                    request_snapshot(&state);
                    if state.settings_save_generation.get() == generation {
                        state.settings_dirty.set(false);
                    } else if state.settings_dirty.get() {
                        let next_generation = state.settings_save_generation.get();
                        schedule_settings_save(&state, next_generation);
                    }
                }
                Err(error) => {
                    show_error(&state, format!("保存设置失败：{error}"));
                }
            }
        },
    );
}

fn apply_theme_from_form(state: &Rc<UiState>) {
    let Some(form) = state.settings_form.borrow().as_ref().cloned() else {
        return;
    };
    let theme = control_combo(&form, "theme");
    let reduced_motion = control_switch(&form, "reduced_motion");
    apply_runtime_settings(&theme, reduced_motion);
}

fn apply_theme_from_settings(state: &Rc<UiState>) {
    if let Some(settings) = state.settings_data.borrow().as_ref() {
        apply_runtime_settings(&settings.theme, settings.reduced_motion);
    }
}

fn apply_runtime_settings(theme: &str, reduced_motion: bool) {
    apply_theme(theme);
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_enable_animations(!reduced_motion);
    }
}

fn apply_theme(theme: &str) {
    let scheme = match theme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

fn start_bangumi_oauth(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| start_bangumi_login(context),
        move |result: Result<BangumiLoginStartData, String>| {
            let Some(state) = weak.upgrade() else { return };
            let Ok(start) = result else {
                show_error(
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
                    show_error(&state, error);
                    return;
                }
            };
            if let Err(error) = oauth::open_default_browser(&start.authorize_url) {
                show_error(&state, format!("无法打开默认浏览器：{error}"));
                return;
            }
            show_success(&state, "已打开 Bangumi 授权页面");
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
                                            show_success(&state, "Bangumi 登录完成，正在同步状态");
                                            request_snapshot(&state);
                                            render_settings(&state);
                                        }
                                        Err(error) => {
                                            show_error(&state, format!("Bangumi 登录失败：{error}"))
                                        }
                                    }
                                }
                            },
                        );
                        glib::ControlFlow::Break
                    }
                    Ok(Err(error)) => {
                        if let Some(state) = weak.upgrade() {
                            show_error(&state, error);
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

fn sync_bangumi_account(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        sync_bangumi_now,
        move |result: Result<crate::service::BangumiSyncSummaryData, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(summary) => {
                        show_success(&state, summary.message);
                        request_snapshot(&state);
                    }
                    Err(error) => show_error(&state, format!("Bangumi 同步失败：{error}")),
                }
            }
        },
    );
}

fn logout_bangumi_account(state: &Rc<UiState>) {
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
                        show_success(&state, "已退出 Bangumi");
                        request_snapshot(&state);
                        render_settings(&state);
                    }
                    Err(error) => show_error(&state, format!("退出 Bangumi 失败：{error}")),
                }
            }
        },
    );
}

fn test_qbittorrent(state: &Rc<UiState>) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        |context| Ok(test_qbittorrent_connection(context)),
        move |result: Result<crate::backend_api::ConnectionTestResponse, String>| {
            if let Some(state) = weak.upgrade() {
                match result {
                    Ok(response) if response.ok => show_success(&state, response.message),
                    Ok(response) => show_error(&state, response.message),
                    Err(error) => show_error(&state, format!("qBittorrent 测试失败：{error}")),
                }
            }
        },
    );
}

fn open_resources(state: &Rc<UiState>, subject: FrontendSubject, episode_number: f64) {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 16);
    container.set_margin_top(24);
    container.set_margin_bottom(28);
    container.set_margin_start(28);
    container.set_margin_end(28);
    container.append(&skeleton::resources());
    let tag = format!("resources-{}", state.next_page_id.get());
    state
        .next_page_id
        .set(state.next_page_id.get().saturating_add(1));
    let resource_view = adw::ToolbarView::new();
    resource_view.add_top_bar(&adw::HeaderBar::new());
    resource_view.set_content(Some(&container));
    let page = adw::NavigationPage::with_tag(&resource_view, "资源", &tag);
    state.navigation.push(&page);
    let request = EpisodeResourcesRequest {
        subject_provider: subject.provider.clone(),
        provider_subject_id: subject.provider_subject_id.clone(),
        title: subject.title.clone(),
        title_cn: subject.title_cn.clone(),
        aliases: subject.aliases.clone(),
        episode_number: Some(episode_number),
        limit: 60,
    };
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| crate::backend_api::episode_resources(context, request),
        move |result: Result<EpisodeResourcesResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            clear_box(&container);
            match result {
                Ok(response) => render_resources(
                    &state,
                    &container,
                    subject,
                    episode_number,
                    response.resources,
                ),
                Err(error) => {
                    container.append(&status("资源搜索失败", &error, "dialog-error-symbolic"))
                }
            }
        },
    );
}

fn render_resources(
    state: &Rc<UiState>,
    container: &gtk::Box,
    subject: FrontendSubject,
    episode_number: f64,
    resources: Vec<crate::service::EpisodeResourceData>,
) {
    let title = format!("资源 · 第 {} 集", episode_number);
    container.append(&page_header(
        &title,
        "可按关键词、清晰度和合集过滤；下载前会打开种子文件选择对话框。",
    ));
    let filters = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("过滤标题或字幕组"));
    search.set_hexpand(true);
    let resolution = gtk::DropDown::from_strings(&["全部清晰度", "1080p", "720p", "2160p"]);
    let sort = gtk::DropDown::from_strings(&["综合排序", "做种数", "发布时间"]);
    let batch = gtk::CheckButton::with_label("仅合集");
    filters.append(&search);
    filters.append(&resolution);
    filters.append(&sort);
    filters.append(&batch);
    container.append(&filters);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    let resources = Rc::new(resources);
    render_resource_rows(
        state,
        &list,
        &resources,
        &subject,
        episode_number,
        "",
        0,
        0,
        false,
    );
    {
        let state = state.clone();
        let list_for_search = list.clone();
        let resources = resources.clone();
        let subject = subject.clone();
        let resolution = resolution.clone();
        let sort = sort.clone();
        let batch = batch.clone();
        search.connect_search_changed(move |entry| {
            render_resource_rows(
                &state,
                &list_for_search,
                &resources,
                &subject,
                episode_number,
                &entry.text(),
                resolution.selected(),
                sort.selected(),
                batch.is_active(),
            )
        });
    }
    {
        let state = state.clone();
        let list_for_resolution = list.clone();
        let resources = resources.clone();
        let subject = subject.clone();
        let search = search.clone();
        let sort = sort.clone();
        let batch = batch.clone();
        resolution.connect_selected_notify(move |dropdown| {
            render_resource_rows(
                &state,
                &list_for_resolution,
                &resources,
                &subject,
                episode_number,
                &search.text(),
                dropdown.selected(),
                sort.selected(),
                batch.is_active(),
            )
        });
    }
    {
        let state = state.clone();
        let list_for_batch = list.clone();
        let resources = resources.clone();
        let subject = subject.clone();
        let search = search.clone();
        let resolution = resolution.clone();
        let sort = sort.clone();
        batch.connect_toggled(move |check| {
            render_resource_rows(
                &state,
                &list_for_batch,
                &resources,
                &subject,
                episode_number,
                &search.text(),
                resolution.selected(),
                sort.selected(),
                check.is_active(),
            )
        });
    }
    {
        let state = state.clone();
        let list_for_sort = list.clone();
        let resources = resources.clone();
        let subject = subject.clone();
        let search = search.clone();
        let resolution = resolution.clone();
        let batch = batch.clone();
        sort.connect_selected_notify(move |dropdown| {
            render_resource_rows(
                &state,
                &list_for_sort,
                &resources,
                &subject,
                episode_number,
                &search.text(),
                resolution.selected(),
                dropdown.selected(),
                batch.is_active(),
            )
        });
    }
    container.append(&scrolled(&list));
}

fn render_resource_rows(
    state: &Rc<UiState>,
    list: &gtk::ListBox,
    resources: &[crate::service::EpisodeResourceData],
    subject: &FrontendSubject,
    episode_number: f64,
    query: &str,
    resolution: u32,
    sort: u32,
    batch_only: bool,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let query = query.trim().to_lowercase();
    let resolution_name = match resolution {
        1 => Some("1080"),
        2 => Some("720"),
        3 => Some("2160"),
        _ => None,
    };
    let mut filtered = resources
        .iter()
        .filter(|resource| {
            (!batch_only || resource.batch)
                && resolution_name.is_none_or(|name| resource.resolution.contains(name))
                && (query.is_empty() || resource.title.to_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    match sort {
        1 => filtered.sort_by(|left, right| {
            right
                .seeders
                .cmp(&left.seeders)
                .then_with(|| right.score.cmp(&left.score))
        }),
        2 => filtered.sort_by(|left, right| {
            right
                .published_at
                .cmp(&left.published_at)
                .then_with(|| right.score.cmp(&left.score))
        }),
        _ => filtered.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| right.seeders.cmp(&left.seeders))
                .then_with(|| left.title.cmp(&right.title))
        }),
    }
    let count = filtered.len();
    for resource in filtered {
        let row = adw::ActionRow::new();
        row.set_title(&resource.title);
        row.set_subtitle(&format!(
            "{} · {} · 做种 {} · 下载 {} · {}",
            if resource.resolution.is_empty() {
                "未知清晰度"
            } else {
                &resource.resolution
            },
            if resource.subtitle_group.is_empty() {
                "未知字幕组"
            } else {
                &resource.subtitle_group
            },
            resource.seeders,
            resource.downloads,
            resource.size,
        ));
        let download = icon_button("folder-download-symbolic", "下载");
        let state_for_download = state.clone();
        let subject_for_download = subject.clone();
        let resource_for_download = resource.clone();
        download.connect_clicked(move |_| {
            prepare_resource(
                &state_for_download,
                subject_for_download.clone(),
                episode_number,
                resource_for_download.clone(),
            )
        });
        row.add_suffix(&download);
        list.append(&row);
    }
    if count == 0 {
        let row = adw::ActionRow::new();
        row.set_title("没有匹配的资源");
        row.set_subtitle("可以清除过滤条件或稍后重试。");
        list.append(&row);
    }
}

fn prepare_resource(
    state: &Rc<UiState>,
    subject: FrontendSubject,
    episode_number: f64,
    resource: crate::service::EpisodeResourceData,
) {
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| {
            crate::backend_api::prepare_resource_download(
                context,
                PrepareResourceDownloadRequest {
                    resource,
                    subject_provider: subject.provider,
                    provider_subject_id: subject.provider_subject_id,
                    episode_number: Some(episode_number),
                },
            )
        },
        move |result: Result<PreparedResourceDownloadResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            match result {
                Ok(prepared) => show_torrent_file_dialog(&state, prepared),
                Err(error) => show_error(&state, format!("准备下载失败：{error}")),
            }
        },
    );
}

fn show_torrent_file_dialog(state: &Rc<UiState>, prepared: PreparedResourceDownloadResponse) {
    let dialog = adw::Dialog::new();
    dialog.set_title("选择种子文件");
    dialog.set_content_width(680);
    dialog.set_content_height(560);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&label(&prepared.task.title, "title-2"));
    content.append(&label(
        "选择要下载的文件，然后确认开始任务。关闭对话框会取消预览任务。",
        "dim-label",
    ));
    let selected = Rc::new(RefCell::new(
        prepared
            .files
            .iter()
            .filter(|file| file.priority > 0)
            .map(|file| file.index)
            .collect::<HashSet<_>>(),
    ));
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    for file in prepared.files.iter() {
        let row = gtk::ListBoxRow::new();
        let check = gtk::CheckButton::with_label(&format!(
            "{} · {}",
            file.name,
            format_bytes_i64(file.size)
        ));
        check.set_active(selected.borrow().contains(&file.index));
        let selected_for_check = selected.clone();
        let index = file.index;
        check.connect_toggled(move |check| {
            if check.is_active() {
                selected_for_check.borrow_mut().insert(index);
            } else {
                selected_for_check.borrow_mut().remove(&index);
            }
        });
        row.set_child(Some(&check));
        list.append(&row);
    }
    content.append(&scrolled(&list));
    let actions = adw::WrapBox::builder()
        .child_spacing(8)
        .line_spacing(8)
        .build();
    actions.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("取消");
    let confirm = action_button("确认下载", "folder-download-symbolic");
    confirm.add_css_class("suggested-action");
    actions.append(&cancel);
    actions.append(&confirm);
    content.append(&actions);
    dialog.set_child(Some(&content));
    let committed = Rc::new(Cell::new(false));
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let dialog = dialog.clone();
        let state = state.clone();
        let selected = selected.clone();
        let task_id = prepared.task.id;
        let committed = committed.clone();
        confirm.connect_clicked(move |_| {
            let indexes = selected.borrow().iter().copied().collect::<Vec<_>>();
            if indexes.is_empty() {
                show_error(&state, "至少选择一个种子文件".to_string());
                return;
            }
            committed.set(true);
            let weak = Rc::downgrade(&state);
            state.runtime.submit(
                move |context| {
                    confirm_resource_download(
                        context,
                        ConfirmResourceDownloadRequest {
                            task_id,
                            selected_file_indexes: indexes,
                        },
                    )
                },
                move |result: Result<crate::service::DownloadTaskData, String>| {
                    if let Some(state) = weak.upgrade() {
                        match result {
                            Ok(_) => {
                                show_success(&state, "下载任务已开始");
                                state.downloads_data.replace(None);
                                state.downloads_requested.set(false);
                                request_downloads(&state);
                            }
                            Err(error) => show_error(&state, format!("确认下载失败：{error}")),
                        }
                    }
                },
            );
            dialog.close();
        });
    }
    {
        let state = state.clone();
        let committed = committed.clone();
        let task_id = prepared.task.id;
        dialog.connect_closed(move |_| {
            if committed.get() {
                return;
            }
            control_download(&state, task_id, "cancel", false);
        });
    }
    dialog.present(Some(&state.window));
}
