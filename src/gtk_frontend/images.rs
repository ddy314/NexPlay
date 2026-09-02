use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gtk::prelude::*;

use super::runtime::BackendRuntime;

#[derive(Clone, Default)]
pub struct ImageLoader {
    cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl ImageLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a standard GTK picture stack.  File paths are loaded directly;
    /// remote images use the same bounded backend pool as other network calls
    /// and are retained in a process-local cache for subsequent page rebuilds.
    pub fn widget(
        &self,
        source: &str,
        runtime: &BackendRuntime,
        width: i32,
        height: i32,
    ) -> gtk::Widget {
        // Keep the allocation stable when a texture arrives.  A bare
        // GtkPicture reports the source image's natural size after loading,
        // which can make an AdwWrapBox reflow a poster from its placeholder
        // size to the source resolution.  AspectFrame provides the fixed
        // poster slot and clips the paintable inside it.
        let frame = gtk::AspectFrame::new(0.5, 0.5, width as f32 / height as f32, false);
        frame.set_size_request(width, height);
        frame.set_hexpand(false);
        frame.set_vexpand(false);
        frame.set_halign(gtk::Align::Start);
        frame.set_valign(gtk::Align::Start);
        frame.set_overflow(gtk::Overflow::Hidden);
        frame.add_css_class("nx-rounded-media");

        let stack = gtk::Stack::new();
        stack.set_size_request(width, height);
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_halign(gtk::Align::Fill);
        stack.set_valign(gtk::Align::Fill);
        stack.set_overflow(gtk::Overflow::Hidden);
        frame.set_child(Some(&stack));

        let placeholder = gtk::Image::from_icon_name("image-x-generic-symbolic");
        placeholder.set_pixel_size(48);
        placeholder.set_hexpand(true);
        placeholder.set_vexpand(true);
        placeholder.set_valign(gtk::Align::Center);
        placeholder.set_halign(gtk::Align::Center);
        stack.add_named(&placeholder, Some("placeholder"));

        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_size_request(width, height);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_halign(gtk::Align::Fill);
        picture.set_valign(gtk::Align::Fill);
        stack.add_named(&picture, Some("image"));
        stack.set_visible_child_name("placeholder");

        // A picture can report the source texture's natural width even when
        // it is allocated into the fixed poster slot above.  Clamp the
        // outer widget as well, otherwise wrapping layouts reserve that
        // larger natural width and leave artificial gaps between cards.
        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(width);
        clamp.set_tightening_threshold(width);
        clamp.set_size_request(width, height);
        clamp.set_hexpand(false);
        clamp.set_vexpand(false);
        clamp.set_halign(gtk::Align::Start);
        clamp.set_valign(gtk::Align::Start);
        clamp.set_child(Some(&frame));

        let source = source.trim().to_string();
        if source.is_empty() {
            return clamp.upcast();
        }

        if let Some(path) = local_path(&source) {
            if path.is_file() {
                picture.set_filename(Some(path));
                stack.set_visible_child_name("image");
            }
            return clamp.upcast();
        }

        if !(source.starts_with("http://") || source.starts_with("https://")) {
            return clamp.upcast();
        }

        if let Some(bytes) = self
            .cache
            .lock()
            .expect("GTK image cache mutex poisoned")
            .get(&source)
            .cloned()
        {
            set_texture(&picture, &stack, bytes);
            return clamp.upcast();
        }

        let cache = self.cache.clone();
        let image_source = source.clone();
        let stack_for_result = stack.clone();
        let picture_for_result = picture.clone();
        runtime.submit(
            move |_| {
                let response = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .build()?
                    .get(&image_source)
                    .send()?
                    .error_for_status()?;
                let bytes = response.bytes()?.to_vec();
                cache
                    .lock()
                    .expect("GTK image cache mutex poisoned")
                    .insert(image_source, bytes.clone());
                Ok(bytes)
            },
            move |result: Result<Vec<u8>, String>| {
                if let Ok(bytes) = result {
                    set_texture(&picture_for_result, &stack_for_result, bytes);
                }
            },
        );
        clamp.upcast()
    }
}

fn local_path(source: &str) -> Option<PathBuf> {
    if let Some(uri) = source.strip_prefix("file://") {
        return reqwest::Url::parse(source)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .or_else(|| Some(PathBuf::from(uri)));
    }
    if source.starts_with('/') || source.starts_with("./") || source.starts_with("../") {
        return Some(PathBuf::from(source));
    }
    None
}

fn set_texture(picture: &gtk::Picture, stack: &gtk::Stack, bytes: Vec<u8>) {
    let bytes = glib::Bytes::from_owned(bytes);
    if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
        picture.set_paintable(Some(&texture));
        stack.set_visible_child_name("image");
    }
}
