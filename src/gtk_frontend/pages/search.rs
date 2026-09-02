use super::super::components::*;
use super::super::prelude::*;
use super::super::state::UiState;
use super::detail::open_subject;

pub(crate) fn search_changed(state: &Rc<UiState>, query: String) {
    let generation = state.search_generation.get().saturating_add(1);
    state.search_generation.set(generation);
    state.search_online.replace(Vec::new());
    state.search_error.replace(None);
    render_search(state);
    if query.trim().chars().count() < 2 {
        return;
    }
    let weak = Rc::downgrade(state);
    state.runtime.submit(
        move |context| search_catalog(context, CatalogSearchRequest { query, limit: 24 }),
        move |result: Result<crate::backend_api::CatalogSearchResponse, String>| {
            let Some(state) = weak.upgrade() else { return };
            if state.search_generation.get() != generation {
                return;
            }
            match result {
                Ok(response) => {
                    state.search_error.replace(None);
                    state.search_online.replace(response.subjects);
                }
                Err(error) => {
                    state.search_error.replace(Some(error));
                }
            }
            render_search(&state);
        },
    );
}

pub(crate) fn render_search(state: &Rc<UiState>) {
    clear_box(&state.search);
    state.search_list.replace(None);
    state.search.append(&page_header(
        "搜索",
        "先过滤本地和云端缓存，输入至少两个字符后在后台延迟查询 Bangumi。Enter 打开选中的条目，Escape 返回。",
    ));
    state.search.append(&state.search_entry);
    let query = state.search_entry.text().to_string().trim().to_lowercase();
    let snapshot = state.snapshot.borrow().clone();
    let mut local = snapshot
        .subjects
        .into_iter()
        .chain(snapshot.bangumi_collections)
        .filter(|subject| subject_matches(subject, &query))
        .collect::<Vec<_>>();
    local = dedupe_subjects(local);
    let online = state.search_online.borrow().clone();
    let results = local.into_iter().chain(online).collect::<Vec<_>>();
    state.search_results.replace(results.clone());
    if query.is_empty() {
        state.search.append(&status(
            "搜索你的资料库",
            "本地结果会即时出现，在线候选只在输入后加载。",
            "system-search-symbolic",
        ));
    } else if results.is_empty() {
        if let Some(error) = state.search_error.borrow().clone() {
            let error_page = status(
                "在线搜索失败",
                &format!("{error}。可以稍后重试。"),
                "dialog-warning-symbolic",
            );
            let retry = action_button("重试搜索", "view-refresh-symbolic");
            let state_for_retry = state.clone();
            let query_for_retry = state.search_entry.text().to_string();
            retry.connect_clicked(move |_| {
                search_changed(&state_for_retry, query_for_retry.clone());
            });
            error_page.set_child(Some(&retry));
            state.search.append(&error_page);
        } else {
            state.search.append(&status(
                "没有匹配条目",
                "可以尝试中文名、日文名、别名或 Bangumi 编号。",
                "system-search-symbolic",
            ));
        }
    } else {
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        for subject in results {
            let row = adw::ActionRow::new();
            row.set_title(&subject_title(&subject));
            row.set_subtitle(&subject_meta(&subject));
            row.set_activatable(true);
            let icon = gtk::Image::from_icon_name(if subject.local {
                "folder-videos-symbolic"
            } else {
                "globe-symbolic"
            });
            row.add_prefix(&icon);
            let state_for_row = state.clone();
            let subject_for_row = subject.clone();
            row.connect_activated(move |_| open_subject(&state_for_row, subject_for_row.clone()));
            list.append(&row);
        }
        list.select_row(list.row_at_index(0).as_ref());
        state.search_list.replace(Some(list.clone()));
        state.search.append(&list);
    }
}

pub(crate) fn subject_matches(subject: &FrontendSubject, query: &str) -> bool {
    query.is_empty()
        || subject.title.to_lowercase().contains(query)
        || subject.title_cn.to_lowercase().contains(query)
        || subject
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(query))
        || subject
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

pub(crate) fn dedupe_subjects(subjects: Vec<FrontendSubject>) -> Vec<FrontendSubject> {
    let mut seen = HashSet::new();
    subjects
        .into_iter()
        .filter(|subject| seen.insert(subject.canonical_key.clone()))
        .collect()
}
