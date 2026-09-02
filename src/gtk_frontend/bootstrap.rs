use super::prelude::*;
use super::{pages::settings_actions, shell, skeleton};

const APP_ID: &str = "dev.nexplay.NexPlay";

pub fn run() -> AppResult<()> {
    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_activate(|application| {
        skeleton::install_css();
        settings_actions::apply_runtime_settings("system", false);
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
                    let root = shell::build_main_ui(context, &window);
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

pub(crate) fn native_config_path() -> PathBuf {
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

pub(crate) fn gtk_config_argument() -> Option<PathBuf> {
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

pub(crate) fn legacy_config_is_more_complete(legacy: &Path, native: &Path) -> bool {
    config_media_library_count(legacy) > config_media_library_count(native)
}

pub(crate) fn config_media_library_count(path: &Path) -> usize {
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

pub(crate) fn native_default_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.database.path = xdg_data_home().join("nexplay").join("nexplay.sqlite3");
    config
}

pub(crate) fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

pub(crate) fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}
