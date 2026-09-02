use gtk::prelude::*;

const SKELETON_CSS: &str = r#"
@keyframes nexplay-skeleton-pulse {
  0%, 100% { opacity: 0.58; }
  50% { opacity: 0.82; }
}

.nx-skeleton {
  background-color: alpha(@window_fg_color, 0.11);
  border-radius: 7px;
  animation: nexplay-skeleton-pulse 1.8s ease-in-out infinite;
}

.nx-skeleton-poster {
  border-radius: 14px;
}

.nx-skeleton-pill,
.nx-skeleton-action {
  border-radius: 999px;
}

.nx-skeleton-control {
  border-radius: 8px;
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

.nx-skeleton-resource-row {
  min-height: 72px;
  padding: 14px 0;
  border-bottom: 1px solid alpha(@window_fg_color, 0.10);
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
    let root = vertical(28);
    root.append(&section_heading(230, 14));
    root.append(&shelf(5));
    root.append(&section_heading(180, 14));
    root.append(&shelf(5));
    root
}

pub fn detail() -> gtk::Box {
    let root = vertical(26);
    let hero = adw::WrapBox::builder()
        .child_spacing(24)
        .line_spacing(24)
        .natural_line_length(900)
        .wrap_policy(adw::WrapPolicy::Minimum)
        .justify(adw::JustifyMode::None)
        .build();
    hero.set_halign(gtk::Align::Fill);
    hero.append(&poster(220, 308));

    // The real detail page clamps this copy column to a readable width.  Keep
    // the loading state in the same column instead of letting each short line
    // inherit the whole window width.
    let copy = vertical(12);
    copy.set_width_request(320);
    copy.set_hexpand(true);
    copy.append(&line(310, 30));
    copy.append(&line(190, 18));
    copy.append(&line(240, 14));
    copy.append(&line(140, 16));
    copy.append(&line(92, 14));

    let tags = horizontal(8);
    tags.set_hexpand(false);
    tags.append(&pill(58, 24));
    tags.append(&pill(72, 24));
    tags.append(&pill(64, 24));
    copy.append(&tags);
    copy.append(&line(480, 13));
    copy.append(&line(420, 13));
    copy.append(&line(270, 13));
    copy.append(&line(250, 12));
    copy.append(&progress(500));

    let copy_width = adw::Clamp::new();
    copy_width.set_maximum_size(600);
    copy_width.set_tightening_threshold(360);
    copy_width.set_halign(gtk::Align::Start);
    copy_width.set_hexpand(false);
    copy_width.set_child(Some(&copy));
    hero.append(&copy_width);
    root.append(&hero);

    root.append(&section_heading(180, 14));
    for (title_width, subtitle_width) in
        [(260, 180), (320, 210), (230, 160), (290, 190), (250, 175)]
    {
        root.append(&episode_row(title_width, subtitle_width));
    }
    root
}

pub fn downloads() -> gtk::Box {
    let root = vertical(0);
    for (title_width, subtitle_width) in [(420, 260), (520, 330), (380, 290), (470, 240)] {
        let row = vertical(10);
        row.add_css_class("nx-skeleton-row");

        let heading = horizontal(10);
        heading.append(&line(title_width, 18));
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        heading.append(&spacer);
        heading.append(&action(30, 30));
        heading.append(&action(30, 30));
        row.append(&heading);
        row.append(&line(subtitle_width, 13));
        // A download progress track is one of the few loading elements that
        // should actually follow the width of its row.
        row.append(&progress(0));
        root.append(&row);
    }
    root
}

pub fn settings() -> gtk::Box {
    let root = vertical(0);
    for (title_width, subtitle_width) in [(240, 380), (210, 320), (260, 360), (190, 290)] {
        let row = horizontal(16);
        row.add_css_class("nx-skeleton-row");
        let text = vertical(8);
        text.set_hexpand(true);
        text.append(&line(title_width, 15));
        text.append(&line(subtitle_width, 11));
        row.append(&text);
        row.append(&control(96, 32));
        root.append(&row);
    }
    root
}

pub fn resources() -> gtk::Box {
    let root = vertical(18);
    root.append(&section_heading(300, 14));

    let filters = horizontal(8);
    filters.set_hexpand(false);
    filters.append(&control(320, 36));
    filters.append(&control(120, 36));
    filters.append(&control(120, 36));
    filters.append(&control(72, 36));
    root.append(&filters);

    for (title_width, subtitle_width) in [
        (560, 390),
        (460, 330),
        (620, 420),
        (500, 360),
        (580, 400),
        (430, 300),
    ] {
        let row = horizontal(14);
        row.add_css_class("nx-skeleton-resource-row");
        let text = vertical(8);
        text.set_hexpand(true);
        text.append(&line(title_width, 16));
        text.append(&line(subtitle_width, 12));
        row.append(&text);
        row.append(&action(32, 32));
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
        card.set_hexpand(false);
        card.set_halign(gtk::Align::Start);
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

fn section_heading(title_width: i32, subtitle_width: i32) -> gtk::Box {
    let heading = vertical(8);
    heading.set_hexpand(false);
    heading.set_halign(gtk::Align::Start);
    heading.append(&line(title_width, 24));
    heading.append(&line(subtitle_width, 13));
    heading
}

fn episode_row(title_width: i32, subtitle_width: i32) -> gtk::Box {
    let row = horizontal(12);
    row.add_css_class("nx-skeleton-row");
    let text = vertical(7);
    text.set_hexpand(true);
    text.append(&line(title_width, 16));
    text.append(&line(subtitle_width, 12));
    row.append(&text);
    row.append(&action(32, 32));
    row
}

fn pill(width: i32, height: i32) -> gtk::Box {
    let pill = block(width, height);
    pill.add_css_class("nx-skeleton-pill");
    pill
}

fn action(width: i32, height: i32) -> gtk::Box {
    let action = block(width, height);
    action.add_css_class("nx-skeleton-action");
    action
}

fn control(width: i32, height: i32) -> gtk::Box {
    let control = block(width, height);
    control.add_css_class("nx-skeleton-control");
    control
}

fn progress(width: i32) -> gtk::Box {
    let progress = block(width.max(1), 8);
    progress.add_css_class("nx-skeleton-progress");
    if width == 0 {
        progress.set_hexpand(true);
        progress.set_halign(gtk::Align::Fill);
    }
    progress
}

fn block(width: i32, height: i32) -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 0);
    block.add_css_class("nx-skeleton");
    // A size request is only a minimum.  Aligning the placeholder to the
    // start keeps a short text-shaped block from filling its parent column.
    block.set_width_request(width);
    block.set_height_request(height);
    block.set_halign(gtk::Align::Start);
    block.set_valign(gtk::Align::Start);
    block
}

fn vertical(spacing: i32) -> gtk::Box {
    let box_widget = gtk::Box::new(gtk::Orientation::Vertical, spacing);
    box_widget.set_hexpand(true);
    box_widget
}

fn horizontal(spacing: i32) -> gtk::Box {
    let box_widget = gtk::Box::new(gtk::Orientation::Horizontal, spacing);
    box_widget.set_hexpand(true);
    box_widget
}
