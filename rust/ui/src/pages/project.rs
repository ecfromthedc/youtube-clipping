use leptos::prelude::*;

#[component]
pub fn ProjectPage(id: String) -> impl IntoView {
    view! { <div class="empty">"Project "{id}" — port in progress. Use the current UI at "<a href="/">"/"</a>" meanwhile."</div> }
}
