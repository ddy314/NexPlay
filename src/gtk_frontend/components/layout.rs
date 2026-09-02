use super::super::prelude::*;

pub(crate) fn page_surface() -> (gtk::ScrolledWindow, gtk::Box) {
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

pub(crate) fn clear_box(box_widget: &gtk::Box) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }
}

pub(crate) fn label(text: impl AsRef<str>, style: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text.as_ref()));
    label.set_xalign(0.0);
    label.set_wrap(true);
    if !style.is_empty() {
        label.add_css_class(style);
    }
    label
}

pub(crate) fn page_header(title: &str, subtitle: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    header.append(&label(title, "title-1"));
    if !subtitle.is_empty() {
        header.append(&label(subtitle, "dim-label"));
    }
    header
}

pub(crate) fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(child));
    scroll
}

pub(crate) fn status(title: &str, description: &str, icon: &str) -> adw::StatusPage {
    let page = adw::StatusPage::new();
    page.set_icon_name(Some(icon));
    page.set_title(title);
    page.set_description(Some(description));
    page.set_vexpand(true);
    page
}

pub(crate) fn action_button(text: &str, _icon: &str) -> gtk::Button {
    // Ordinary content buttons stay label-only.  Icon-only actions use
    // `icon_button`, which keeps the button hierarchy compact and follows the
    // GNOME button guidance outside a header bar.
    gtk::Button::with_label(text)
}

pub(crate) fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button
}

pub(crate) fn adaptive_wrap() -> adw::WrapBox {
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

pub(crate) fn append_button_row(
    container: &gtk::Box,
    title: &str,
    subtitle: &str,
    button: &gtk::Button,
) {
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
