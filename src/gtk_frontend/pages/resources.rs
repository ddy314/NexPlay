use super::super::components::*;
use super::super::prelude::*;
use super::super::{skeleton, state::UiState};
use super::resource_download;

pub(crate) fn open_resources(state: &Rc<UiState>, subject: FrontendSubject, episode_number: f64) {
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

pub(crate) fn render_resources(
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

pub(crate) fn render_resource_rows(
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
            resource_download::prepare_resource(
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
