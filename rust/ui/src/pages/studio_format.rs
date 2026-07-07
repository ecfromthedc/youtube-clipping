use leptos::prelude::*;

#[component]
pub fn StudioFormat(slug: String) -> impl IntoView {
    view! { <div class="empty">"Studio format "{slug}" — port in progress. Use the current UI at "<a href="/">"/"</a>" meanwhile."</div> }
}
