use super::super::prelude::*;
use super::super::{pages::detail, state::UiState};
use super::layout::{adaptive_wrap, label};

pub(crate) fn subject_title(subject: &FrontendSubject) -> String {
    if subject.title_cn.trim().is_empty() {
        subject.title.trim().to_string()
    } else {
        subject.title_cn.trim().to_string()
    }
}

pub(crate) fn subject_meta(subject: &FrontendSubject) -> String {
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

pub(crate) fn subject_card(state: &Rc<UiState>, subject: FrontendSubject) -> gtk::Box {
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
    poster_button.connect_clicked(move |_| detail::open_subject(&state_for_click, subject.clone()));
    item
}

pub(crate) fn subject_shelf(
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
