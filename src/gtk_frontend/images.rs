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
        let stack = gtk::Stack::new();
        stack.set_size_request(width, height);

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
        stack.add_named(&picture, Some("image"));
        stack.set_visible_child_name("placeholder");

        let source = source.trim().to_string();
        if source.is_empty() {
            return stack.upcast();
        }

        if let Some(path) = local_path(&source) {
            if path.is_file() {
                picture.set_filename(Some(path));
                stack.set_visible_child_name("image");
            }
            return stack.upcast();
        }

        if !(source.starts_with("http://") || source.starts_with("https://")) {
            return stack.upcast();
        }

        if let Some(bytes) = self
            .cache
            .lock()
            .expect("GTK image cache mutex poisoned")
            .get(&source)
            .cloned()
        {
            set_texture(&picture, &stack, bytes);
            return stack.upcast();
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
        stack.upcast()
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
