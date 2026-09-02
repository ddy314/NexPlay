use gtk::prelude::*;

const SKELETON_CSS: &str = r#"
@keyframes nexplay-skeleton-pulse {
  from { opacity: 0.48; }
  to { opacity: 0.92; }
}

.nx-skeleton {
  background-color: alpha(@window_fg_color, 0.10);
  border-radius: 6px;
  animation: nexplay-skeleton-pulse 1.4s ease-in-out infinite alternate;
}

.nx-skeleton-poster {
  border-radius: 14px;
}

.nx-skeleton-row {
  min-height: 68px;
  padding: 14px 0;
  border-bottom: 1px solid alpha(@window_fg_color, 0.10);
}

.nx-skeleton-progress {
  min-height: 8px;
  border-radius: 4px;
}

.nx-rounded-media {
  border-radius: 14px;
  overflow: hidden;
}

.nx-media-card {
  border-radius: 14px;
  padding: 0;
}

.nx-tag {
  padding: 3px 8px;
  border-radius: 999px;
  background-color: alpha(@window_fg_color, 0.08);
}

.nx-download-row {
  background-color: transparent;
  padding: 16px 0;
  border-bottom: 1px solid alpha(@window_fg_color, 0.10);
}

.nx-episode-list,
.nx-download-list {
  background-color: transparent;
}

.nx-episode-list > row,
.nx-download-list > row {
  background-color: transparent;
}

.nx-episode-list > row:hover,
.nx-download-list > row:hover {
  background-color: alpha(@window_fg_color, 0.06);
}

.nx-episode-row {
  min-height: 68px;
  padding: 10px 0;
  background-color: transparent;
  border-bottom: 1px solid alpha(@window_fg_color, 0.10);
}

.nx-episode-row > button {
  background-color: transparent;
}
"#;

pub fn install_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(SKELETON_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn home() -> gtk::Box {
    let root = vertical(18);
    root.append(&line(230, 32));
    root.append(&line(360, 18));
    root.append(&shelf(5));
    root.append(&line(180, 26));
    root.append(&line(300, 16));
    root.append(&shelf(5));
    root
}

pub fn detail() -> gtk::Box {
    let root = vertical(22);
    let hero = adw::WrapBox::builder()
        .child_spacing(24)
        .line_spacing(24)
        .natural_line_length(900)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    hero.set_halign(gtk::Align::Fill);
    hero.append(&poster(220, 308));
    let copy = vertical(12);
    copy.set_width_request(420);
    copy.set_hexpand(true);
    copy.append(&line(250, 30));
    copy.append(&line(170, 18));
    copy.append(&line(230, 14));
    copy.append(&line(150, 16));
    copy.append(&line(390, 42));
    copy.append(&line(330, 14));
    copy.append(&line(500, 8));
    hero.append(&copy);
    root.append(&hero);
    root.append(&line(110, 24));
    root.append(&line(300, 14));
    for _ in 0..5 {
        let row = vertical(8);
        row.add_css_class("nx-skeleton-row");
        row.append(&line(260, 16));
        row.append(&line(180, 12));
        root.append(&row);
    }
    root
}

pub fn downloads() -> gtk::Box {
    let root = vertical(0);
    for _ in 0..4 {
        let row = vertical(10);
        row.add_css_class("nx-skeleton-row");
        row.append(&line(520, 18));
        row.append(&line(360, 13));
        let progress = line(680, 8);
        progress.add_css_class("nx-skeleton-progress");
        row.append(&progress);
        root.append(&row);
    }
    root
}

pub fn settings() -> gtk::Box {
    let root = vertical(0);
    for _ in 0..4 {
        let row = vertical(8);
        row.add_css_class("nx-skeleton-row");
        row.append(&line(240, 15));
        row.append(&line(380, 11));
        root.append(&row);
    }
    root
}

fn shelf(count: usize) -> adw::WrapBox {
    let shelf = adw::WrapBox::builder()
        .child_spacing(18)
        .line_spacing(18)
        .natural_line_length(1120)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    for _ in 0..count {
        let card = vertical(8);
        card.set_width_request(160);
        card.append(&poster(160, 226));
        card.append(&line(132, 16));
        card.append(&line(92, 12));
        shelf.append(&card);
    }
    shelf
}

fn poster(width: i32, height: i32) -> gtk::Box {
    let block = block(width, height);
    block.add_css_class("nx-skeleton-poster");
    block
}

fn line(width: i32, height: i32) -> gtk::Box {
    block(width, height)
}

fn block(width: i32, height: i32) -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 0);
    block.add_css_class("nx-skeleton");
    block.set_size_request(width, height);
    block
}

fn vertical(spacing: i32) -> gtk::Box {
    let box_widget = gtk::Box::new(gtk::Orientation::Vertical, spacing);
    box_widget.set_hexpand(true);
    box_widget
}
