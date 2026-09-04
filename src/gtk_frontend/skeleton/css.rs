pub(crate) const SKELETON_CSS: &str = r#"
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
}

.nx-source-switch {
  min-width: 96px;
  min-height: 36px;
}

.nx-poster-placeholder {
  border-radius: 14px;
  background-color: alpha(@window_fg_color, 0.08);
}

.nx-poster-placeholder label {
  font-size: 12px;
  font-weight: 600;
}

/*
 * The poster is the interactive surface.  Keep the title and metadata as
 * ordinary content below it instead of turning the whole media item into a
 * large button/card.  The hover treatment is an image overlay and play
 * affordance; it deliberately has no border or shadow.
 */
.nx-poster-button {
  min-width: 0;
  min-height: 0;
  padding: 0;
  border-radius: 14px;
  background-color: transparent;
  background-image: none;
  box-shadow: none;
}

.nx-poster-button:hover,
.nx-poster-button:active {
  background-color: transparent;
  background-image: none;
  box-shadow: none;
}

.nx-poster-hover {
  border-radius: 14px;
  background-color: transparent;
  transition: background-color 180ms ease;
}

.nx-poster-play {
  opacity: 0;
  transition: opacity 180ms ease;
}

.nx-poster-button:hover .nx-poster-hover {
  background-color: alpha(@window_fg_color, 0.10);
}

.nx-poster-button:active .nx-poster-hover {
  background-color: alpha(@window_fg_color, 0.18);
}

.nx-poster-button:hover .nx-poster-play,
.nx-poster-button:active .nx-poster-play {
  opacity: 1;
}

.nx-poster-play {
  color: @window_fg_color;
  -gtk-icon-shadow: 0 1px 8px alpha(@window_bg_color, 0.70);
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
