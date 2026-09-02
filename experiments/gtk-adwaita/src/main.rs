use std::path::PathBuf;

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "dev.nexplay.NativePrototype";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let _ = adw::init();

    let stack = adw::ViewStack::new();
    stack.set_enable_transitions(true);
    stack.set_transition_duration(220);

    let focus_page = build_focus_page();
    let library_page = build_library_page();
    let settings_page = build_settings_page();

    let focus_stack_page = stack.add_titled(&focus_page, Some("focus"), "Focus");
    focus_stack_page.set_icon_name(Some("starred-symbolic"));
    let library_stack_page = stack.add_titled(&library_page, Some("library"), "Library");
    library_stack_page.set_icon_name(Some("folder-videos-symbolic"));
    let settings_stack_page = stack.add_titled(&settings_page, Some("settings"), "Settings");
    settings_stack_page.set_icon_name(Some("emblem-system-symbolic"));

    let sidebar_switcher = adw::ViewSwitcherSidebar::new();
    sidebar_switcher.set_stack(Some(&stack));

    let sidebar_toolbar = adw::ToolbarView::new();
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.set_title_widget(Some(&brand_title()));
    sidebar_toolbar.add_top_bar(&sidebar_header);

    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.append(&sidebar_switcher);
    sidebar_toolbar.set_content(Some(&sidebar_box));
    let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "NexPlay");

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&stack));

    let content_toolbar = adw::ToolbarView::new();
    let content_header = adw::HeaderBar::new();
    let prototype_label = gtk::Label::new(Some("STATIC PROTOTYPE"));
    prototype_label.add_css_class("caption");
    prototype_label.add_css_class("dim-label");
    content_header.set_title_widget(Some(&prototype_label));

    let about_button = gtk::Button::from_icon_name("help-about-symbolic");
    about_button.set_tooltip_text(Some("About this experiment"));
    let toast_for_about = toast_overlay.clone();
    about_button.connect_clicked(move |_| {
        toast_for_about.add_toast(adw::Toast::new(
            "Native libadwaita surfaces — no custom widgets or CSS",
        ));
    });
    content_header.pack_end(&about_button);
    content_toolbar.add_top_bar(&content_header);
    content_toolbar.set_content(Some(&toast_overlay));

    let content_page = adw::NavigationPage::new(&content_toolbar, "NexPlay");
    let split_view = adw::NavigationSplitView::new();
    split_view.set_sidebar(Some(&sidebar_page));
    split_view.set_content(Some(&content_page));
    split_view.set_min_sidebar_width(220.0);
    split_view.set_max_sidebar_width(280.0);
    split_view.set_sidebar_width_fraction(0.24);

    let window = adw::ApplicationWindow::new(app);
    window.set_title(Some("NexPlay — GTK4 prototype"));
    window.set_default_size(1320, 860);
    window.set_content(Some(&split_view));
    window.present();
}

fn brand_title() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon = gtk::Image::from_icon_name("multimedia-player-symbolic");
    icon.set_pixel_size(20);
    let label = gtk::Label::new(Some("NexPlay"));
    label.add_css_class("title-4");
    box_.append(&icon);
    box_.append(&label);
    box_
}

fn build_focus_page() -> gtk::Widget {
    let toast_overlay = adw::ToastOverlay::new();
    let banner = adw::Banner::new("Static prototype — controls demonstrate native feedback only");
    banner.set_revealed(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 24);
    content.set_margin_top(24);
    content.set_margin_bottom(36);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let eyebrow = gtk::Label::new(Some("TONIGHT'S FOCUS"));
    eyebrow.set_xalign(0.0);
    eyebrow.add_css_class("caption");
    eyebrow.add_css_class("accent");
    let title = gtk::Label::new(Some("A quiet queue, one deliberate choice."));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    let subtitle = gtk::Label::new(Some(
        "Adwaita keeps the content calm and lets the system own hierarchy, spacing, focus, and adaptation.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    heading.append(&eyebrow);
    heading.append(&title);
    heading.append(&subtitle);
    content.append(&heading);

    let (hero, carousel, second_slide) = build_carousel();
    let play_button = gtk::Button::from_icon_name("media-playback-start-symbolic");
    play_button.add_css_class("suggested-action");
    play_button.add_css_class("circular");
    play_button.set_tooltip_text(Some("Preview the native carousel transition"));
    let carousel_for_button = carousel.clone();
    let toast_for_button = toast_overlay.clone();
    play_button.connect_clicked(move |_| {
        carousel_for_button.scroll_to(&second_slide, true);
        toast_for_button.add_toast(adw::Toast::new("Preview state changed"));
    });
    hero.add_overlay(&play_button);
    play_button.set_halign(gtk::Align::End);
    play_button.set_valign(gtk::Align::End);
    play_button.set_margin_end(20);
    play_button.set_margin_bottom(20);
    content.append(&hero);

    let indicators = adw::CarouselIndicatorDots::new();
    indicators.set_carousel(Some(&carousel));
    indicators.set_halign(gtk::Align::Center);
    content.append(&indicators);

    let signals = adw::PreferencesGroup::builder()
        .title("Signals")
        .description("Adwaita rows keep status readable without a second card language.")
        .build();
    signals.add(&signal_row(
        "06h 42m",
        "Watched this week",
        "+18% from last week",
        "document-open-recent-symbolic",
    ));
    signals.add(&signal_row(
        "Low",
        "Queue energy",
        "A calm next step",
        "weather-clear-symbolic",
    ));
    signals.add(&signal_row(
        "94%",
        "Library signal",
        "Metadata in agreement",
        "emblem-ok-symbolic",
    ));
    content.append(&signals);

    let next_group = adw::PreferencesGroup::builder()
        .title("Considered next step")
        .description("The interaction is deliberately small: select, preview, continue.")
        .build();
    next_group.add(&action_row(
        "Continue from where the room gets quiet",
        "Episode 04 · 16 minutes remaining · local",
        "media-playback-start-symbolic",
    ));
    let sync_switch = gtk::Switch::new();
    sync_switch.set_active(true);
    let sync_row = adw::ActionRow::builder()
        .title("Quiet sync")
        .subtitle("Native switch, no confirmation ceremony")
        .activatable_widget(&sync_switch)
        .build();
    sync_row.add_prefix(&gtk::Image::from_icon_name("emblem-synchronizing-symbolic"));
    sync_row.add_suffix(&sync_switch);
    next_group.add(&sync_row);
    content.append(&next_group);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1120);
    clamp.set_tightening_threshold(840);
    clamp.set_child(Some(&content));
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&clamp));

    let page_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page_box.append(&banner);
    page_box.append(&scroll);
    toast_overlay.set_child(Some(&page_box));
    toast_overlay.upcast()
}

fn build_carousel() -> (gtk::Overlay, adw::Carousel, gtk::Widget) {
    let carousel = adw::Carousel::new();
    carousel.set_allow_mouse_drag(true);
    carousel.set_allow_scroll_wheel(true);
    carousel.set_interactive(true);
    carousel.set_reveal_duration(420);
    carousel.set_hexpand(true);
    carousel.set_height_request(318);

    let first = build_hero_slide(
        "The quiet before the blue hour",
        "Episode 04 · 23 minutes · 68% familiar",
        "A 23-minute pick for the end of the day — atmospheric, unhurried, and already in reach.",
        "Continue preview",
    );
    let second = build_hero_slide(
        "A little room for tomorrow",
        "Saved note · 3 minutes to revisit",
        "The same content can become a lightweight decision surface when the surrounding chrome stays quiet.",
        "Open note",
    );
    carousel.append(&first);
    carousel.append(&second);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&carousel));
    (overlay, carousel, second.upcast())
}

fn build_hero_slide(title: &str, meta: &str, description: &str, action: &str) -> gtk::Box {
    let slide = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    slide.add_css_class("card");

    let picture = gtk::Picture::for_filename(cover_path());
    picture.set_content_fit(gtk::ContentFit::Cover);
    picture.set_can_shrink(true);
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_size_request(270, -1);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 12);
    copy.set_hexpand(true);
    copy.set_valign(gtk::Align::Center);
    copy.set_margin_start(28);
    copy.set_margin_end(28);
    copy.set_margin_top(24);
    copy.set_margin_bottom(24);

    let label = gtk::Label::new(Some("NEXT UP  /  EPISODE 04"));
    label.set_xalign(0.0);
    label.add_css_class("caption");
    label.add_css_class("accent");
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.set_wrap(true);
    title_label.add_css_class("title-2");
    let meta_label = gtk::Label::new(Some(meta));
    meta_label.set_xalign(0.0);
    meta_label.add_css_class("dim-label");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("dim-label");
    let action_button = gtk::Button::with_label(action);
    action_button.add_css_class("flat");
    action_button.set_halign(gtk::Align::Start);
    copy.append(&label);
    copy.append(&title_label);
    copy.append(&meta_label);
    copy.append(&description_label);
    copy.append(&action_button);

    slide.append(&picture);
    slide.append(&copy);
    slide
}

fn build_library_page() -> gtk::Widget {
    let status = adw::StatusPage::new();
    status.set_icon_name(Some("folder-videos-symbolic"));
    status.set_title("A shelf with a point of view");
    status.set_description(Some(
        "The library probe is intentionally empty of backend data. Adwaita's status page makes that state clear without inventing a custom empty-state component.",
    ));
    status.set_vexpand(true);
    status.set_margin_start(32);
    status.set_margin_end(32);
    status.add_css_class("compact");
    status.upcast()
}

fn build_settings_page() -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    page.set_vexpand(true);

    let behavior = adw::PreferencesGroup::builder()
        .title("Behavior")
        .description("Adwaita rows establish the settings rhythm and keep controls aligned.")
        .build();
    let motion = gtk::Switch::new();
    motion.set_active(true);
    let motion_row = adw::ActionRow::builder()
        .title("Use ambient transitions")
        .subtitle("Short, interruptible motion for page and carousel changes")
        .activatable_widget(&motion)
        .build();
    motion_row.add_prefix(&gtk::Image::from_icon_name(
        "media-playlist-consecutive-symbolic",
    ));
    motion_row.add_suffix(&motion);
    behavior.add(&motion_row);

    let reduced = gtk::Switch::new();
    let reduced_row = adw::ActionRow::builder()
        .title("Reduced motion")
        .subtitle("A real product setting would hand this to the animation policy")
        .activatable_widget(&reduced)
        .build();
    reduced_row.add_prefix(&gtk::Image::from_icon_name("accessibility-symbolic"));
    reduced_row.add_suffix(&reduced);
    behavior.add(&reduced_row);
    page.add(&behavior);

    let appearance = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("No custom stylesheet is loaded in this prototype.")
        .build();
    appearance.add(&action_row(
        "System color scheme",
        "Follow the desktop preference",
        "weather-clear-symbolic",
    ));
    appearance.add(&action_row(
        "Accent color",
        "Adwaita default accent",
        "color-select-symbolic",
    ));
    page.add(&appearance);

    page.upcast()
}

fn signal_row(value: &str, title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let value_label = gtk::Label::new(Some(value));
    value_label.add_css_class("numeric");
    value_label.add_css_class("title-3");
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(icon_name));
    row.add_suffix(&value_label);
    row
}

fn action_row(title: &str, subtitle: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(icon_name));
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    row
}

fn cover_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/aurora-cover.svg")
}
