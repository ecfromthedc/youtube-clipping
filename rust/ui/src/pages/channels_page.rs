//! Channels workspace — each connected account is a content folder.
//!
//! Eric's spec: "folders mirror the accounts we have connected so the content
//! we make can just get stored in that corresponding account… then we can
//! decide to deploy it into the optimal youtube time slots."
//!
//! Rail of connected channels (toggle) → per-channel: Postiz mapping (explicit,
//! shared-account guardrail), posting-slot times (ET, research defaults),
//! the channel's content library, an unassigned bucket to file from, and the
//! upcoming queue. "Deploy" picks one of the next OPEN slots and schedules via
//! the existing /api/postiz/publish (schedule + date).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::http::{get_json, post_json};

// ── API shapes ────────────────────────────────────────────────────────────────

#[derive(Clone, Deserialize)]
struct ChannelsResp {
    #[serde(default)]
    channels: Vec<Channel>,
}
#[derive(Clone, Deserialize, PartialEq)]
struct Channel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    postiz_integration_id: Option<String>,
    #[serde(default)]
    slot_times: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct LibraryResp {
    #[serde(default)]
    items: Vec<Item>,
}
#[derive(Clone, Deserialize, PartialEq)]
struct Item {
    #[serde(default)]
    project: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    published: bool,
}

#[derive(Clone, Deserialize)]
struct SlotsResp {
    #[serde(default)]
    slots: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct QueueResp {
    #[serde(default)]
    mapped: bool,
    #[serde(default)]
    queue: Vec<QueueItem>,
}
#[derive(Clone, Deserialize, PartialEq)]
struct QueueItem {
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Clone, Deserialize)]
struct IntegrationsResp {
    #[serde(default)]
    integrations: Vec<Integration>,
}
#[derive(Clone, Deserialize, PartialEq)]
struct Integration {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    identifier: String,
}

#[derive(Serialize)]
struct AssignReq {
    project: String,
    kind: String,
    file: String,
    channel: String,
}

#[derive(Serialize)]
struct PublishReq {
    path: String,
    integration_id: String,
    title: String,
    schedule: String,
    date: String,
}

#[derive(Serialize)]
struct MapReq {
    integration_id: String,
}

#[derive(Serialize)]
struct SlotsReq {
    times: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct OkResp {
    #[serde(default)]
    #[allow(dead_code)]
    ok: bool,
}

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "compiles" => "🏆",
        "stories" => "📖",
        "commentary" => "🎬",
        _ => "✂️",
    }
}

/// "2026-07-08T07:00:00-04:00" → "Tue Jul 8 · 7:00 AM ET" (Mon-Rovia-style label).
/// Hand-parsed (fixed RFC3339 from our own server, no chrono in wasm) so the
/// wall-clock shown is the slot's OWN timezone (ET), not the viewer's.
fn slot_label(iso: &str) -> String {
    let b = iso.as_bytes();
    if b.len() < 16 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return iso.to_string();
    }
    let num = |r: std::ops::Range<usize>| iso[r].parse::<i64>().unwrap_or(0);
    let (y, mo, d, h, mi) = (num(0..4), num(5..7), num(8..10), num(11..13), num(14..16));
    // Zeller's congruence → day of week (0 = Saturday).
    let (zm, zy) = if mo < 3 { (mo + 12, y - 1) } else { (mo, y) };
    let z = (d + 13 * (zm + 1) / 5 + zy % 100 + zy % 100 / 4 + zy / 100 / 4 + 5 * (zy / 100)) % 7;
    let dow = ["Sat", "Sun", "Mon", "Tue", "Wed", "Thu", "Fri"][z as usize];
    let mon = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(mo - 1).clamp(0, 11) as usize];
    let (h12, ap) = (
        if h % 12 == 0 { 12 } else { h % 12 },
        if h >= 12 { "PM" } else { "AM" },
    );
    format!("{dow} {mon} {d} · {h12}:{mi:02} {ap} ET")
}

#[component]
pub fn ChannelsPage() -> impl IntoView {
    let channels = RwSignal::new(Vec::<Channel>::new());
    let selected = RwSignal::new(None::<String>);
    let library = RwSignal::new(Vec::<Item>::new());
    let queue = RwSignal::new(None::<QueueResp>);
    let integrations = RwSignal::new(Vec::<Integration>::new());
    let status = RwSignal::new(String::new()); // page-level action feedback (loud)
    let bump = RwSignal::new(0u32); // refetch trigger

    let reload = move || bump.update(|b| *b += 1);

    Effect::new(move |_| {
        bump.get();
        spawn_local(async move {
            if let Ok(r) = get_json::<ChannelsResp>("/api/channels").await {
                if selected.get_untracked().is_none() {
                    if let Some(first) = r.channels.first() {
                        selected.set(Some(first.id.clone()));
                    }
                }
                let _ = channels.try_set(r.channels);
            }
            if let Ok(r) = get_json::<LibraryResp>("/api/library").await {
                let _ = library.try_set(r.items);
            }
            if let Ok(r) = get_json::<IntegrationsResp>("/api/postiz/integrations").await {
                let _ = integrations.try_set(r.integrations);
            }
        });
    });
    // Queue follows the selected channel.
    Effect::new(move |_| {
        bump.get();
        let Some(id) = selected.get() else { return };
        spawn_local(async move {
            let r = get_json::<QueueResp>(&format!("/api/channels/{id}/queue"))
                .await
                .ok();
            let _ = queue.try_set(r);
        });
    });

    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">"Channels"</h1>
                <p class="page-sub">
                    "Every connected account is a folder. File renders into a channel, then deploy them into its next optimal YouTube slots."
                </p>
            </div>
            <div class="row">
                <a class="btn btn-sm" href="/api/oauth/yt/start">"＋ Connect a channel"</a>
            </div>
        </div>
        {move || {
            let st = status.get();
            (!st.is_empty())
                .then(|| {
                    view! {
                        <div class="alert alert-info mb-16" style="display:block">{st}</div>
                    }
                })
        }}
        {move || {
            let list = channels.get();
            if list.is_empty() {
                return view! {
                    <div class="alert alert-warn" style="display:block">
                        "No channels connected yet. Click "
                        <a href="/api/oauth/yt/start">"＋ Connect a channel"</a>
                        " and approve with the channel's Google login — it becomes a content folder here."
                    </div>
                }
                    .into_any();
            }
            let active_id = selected.get().unwrap_or_else(|| list[0].id.clone());
            let active = list
                .iter()
                .find(|c| c.id == active_id)
                .cloned()
                .unwrap_or_else(|| list[0].clone());
            view! {
                <div class="row mb-24" style="flex-wrap: wrap; gap: 8px;">
                    {list
                        .iter()
                        .map(|c| {
                            let id = c.id.clone();
                            let is_active = c.id == active.id;
                            view! {
                                <button
                                    class=if is_active { "btn btn-sm" } else { "btn btn-ghost btn-sm" }
                                    on:click=move |_| selected.set(Some(id.clone()))
                                >
                                    {format!("📺 {}", c.title)}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                {channel_panel(
                    active,
                    library,
                    queue,
                    integrations,
                    status,
                    reload,
                )}
            }
                .into_any()
        }}
    }
}

fn channel_panel(
    ch: Channel,
    library: RwSignal<Vec<Item>>,
    queue: RwSignal<Option<QueueResp>>,
    integrations: RwSignal<Vec<Integration>>,
    status: RwSignal<String>,
    reload: impl Fn() + Copy + Send + Sync + 'static,
) -> AnyView {
    let ch_id = StoredValue::new(ch.id.clone());
    let mapped = ch.postiz_integration_id.clone();
    let slots_csv = RwSignal::new(ch.slot_times.join(", "));
    // Which library item has its slot picker open + the fetched open slots.
    let picking = RwSignal::new(None::<String>);
    let open_slots = RwSignal::new(Vec::<String>::new());

    let map_view = match &mapped {
        Some(iid) => {
            let iid = iid.clone();
            view! {
                <span class="pill">{format!("🔗 Postiz mapped · {}", &iid[..iid.len().min(10)])}</span>
            }
                .into_any()
        }
        None => {
            let sel: NodeRef<leptos::html::Select> = NodeRef::new();
            view! {
                <div class="row" style="gap:8px; flex-wrap: wrap;">
                    <span class="pill pill-warn">"not mapped to Postiz"</span>
                    <select class="pa-input" style="width:auto" node_ref=sel>
                        <option value="">"— pick THIS channel's integration —"</option>
                        {move || {
                            integrations
                                .get()
                                .into_iter()
                                .map(|i| {
                                    view! {
                                        <option value=i
                                            .id
                                            .clone()>
                                            {format!("{} ({})", i.name, i.identifier)}
                                        </option>
                                    }
                                })
                                .collect_view()
                        }}
                    </select>
                    <button
                        class="btn btn-sm"
                        on:click=move |_| {
                            let Some(el) = sel.get_untracked() else { return };
                            let iid = el.value();
                            if iid.is_empty() {
                                return;
                            }
                            let confirmed = web_sys::window()
                                .map(|w| {
                                    w.confirm_with_message(
                                            "Map this integration to this channel?\n\n⛔ SHARED Postiz \
                                             account: only map an integration YOUR team owns for THIS \
                                             channel — never a teammate's.",
                                        )
                                        .unwrap_or(false)
                                })
                                .unwrap_or(false);
                            if !confirmed {
                                return;
                            }
                            let id = ch_id.get_value();
                            spawn_local(async move {
                                match post_json::<
                                    MapReq,
                                    OkResp,
                                >(
                                        &format!("/api/channels/{id}/map-postiz"),
                                        &MapReq { integration_id: iid },
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        status.set("✓ Postiz integration mapped".into());
                                        reload();
                                    }
                                    Err(e) => status.set(format!("⚠ mapping failed: {e}")),
                                }
                            });
                        }
                    >
                        "Map"
                    </button>
                </div>
            }
                .into_any()
        }
    };

    let assigned: Vec<Item> = library
        .get()
        .into_iter()
        .filter(|i| i.channel.as_deref() == Some(ch.id.as_str()))
        .collect();
    let unassigned: Vec<Item> = library
        .get()
        .into_iter()
        .filter(|i| i.channel.is_none())
        .collect();
    let mapped_for_rows = mapped.clone();

    view! {
        <div class="panel mb-16">
            <div class="row-between mb-8">
                <strong>"Account"</strong>
                {map_view}
            </div>
            <div class="row" style="gap:8px; flex-wrap: wrap; align-items: center;">
                <span class="muted">"Posting slots (ET):"</span>
                <input
                    class="pa-input"
                    style="width: 220px"
                    prop:value=move || slots_csv.get()
                    on:input=move |e| slots_csv.set(event_target_value(&e))
                />
                <button
                    class="btn btn-ghost btn-sm"
                    on:click=move |_| {
                        let times: Vec<String> = slots_csv
                            .get_untracked()
                            .split(',')
                            .map(|t| t.trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();
                        let id = ch_id.get_value();
                        spawn_local(async move {
                            match post_json::<
                                SlotsReq,
                                OkResp,
                            >(&format!("/api/channels/{id}/slots"), &SlotsReq { times })
                                .await
                            {
                                Ok(_) => {
                                    status.set("✓ slot times saved".into());
                                    reload();
                                }
                                Err(e) => status.set(format!("⚠ slots: {e}")),
                            }
                        });
                    }
                >
                    "Save"
                </button>
                <span class="muted" style="font-size:11px">
                    "defaults 07:00 / 12:15 / 19:30 — ahead of the pre-commute, lunch, and evening scroll peaks; the loop refines from real data"
                </span>
            </div>
        </div>

        <div class="panel mb-16">
            <div class="row-between mb-8">
                <strong>{format!("📁 {} — library ({})", ch.title, assigned.len())}</strong>
            </div>
            {if assigned.is_empty() {
                view! {
                    <div class="muted">
                        "Nothing filed here yet — assign renders from the bucket below."
                    </div>
                }
                    .into_any()
            } else {
                assigned
                    .into_iter()
                    .map(|item| {
                        library_row(
                            item,
                            ch_id,
                            mapped_for_rows.clone(),
                            picking,
                            open_slots,
                            status,
                            reload,
                            true,
                        )
                    })
                    .collect_view()
                    .into_any()
            }}
        </div>

        {move || {
            let q = queue.get();
            let rows = q.as_ref().map(|q| q.queue.clone()).unwrap_or_default();
            let is_mapped = q.as_ref().map(|q| q.mapped).unwrap_or(false);
            view! {
                <div class="panel mb-16">
                    <div class="row-between mb-8">
                        <strong>{format!("📅 Upcoming queue ({})", rows.len())}</strong>
                    </div>
                    {if !is_mapped {
                        view! {
                            <div class="muted">"Map the Postiz integration to see + build the queue."</div>
                        }
                            .into_any()
                    } else if rows.is_empty() {
                        view! { <div class="muted">"Queue is empty — deploy something 👇"</div> }
                            .into_any()
                    } else {
                        rows.into_iter()
                            .map(|p| {
                                view! {
                                    <div class="row-between mb-8">
                                        <span>
                                            {p.title.clone().unwrap_or_else(|| "untitled".into())}
                                        </span>
                                        <span class="muted mono">
                                            {p.date.as_deref().map(slot_label).unwrap_or_default()}
                                            {p.state
                                                .as_deref()
                                                .map(|s| format!(" · {s}"))
                                                .unwrap_or_default()}
                                        </span>
                                    </div>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
            }
        }}

        <div class="panel">
            <div class="row-between mb-8">
                <strong>{format!("🗃 Unassigned renders ({})", unassigned.len())}</strong>
                <span class="muted">"file them into a channel folder"</span>
            </div>
            {if unassigned.is_empty() {
                view! { <div class="muted">"Everything is filed. Clean desk."</div> }.into_any()
            } else {
                unassigned
                    .into_iter()
                    .map(|item| {
                        library_row(
                            item,
                            ch_id,
                            mapped_for_rows.clone(),
                            picking,
                            open_slots,
                            status,
                            reload,
                            false,
                        )
                    })
                    .collect_view()
                    .into_any()
            }}
        </div>
    }
    .into_any()
}

#[allow(clippy::too_many_arguments)]
fn library_row(
    item: Item,
    ch_id: StoredValue<String>,
    mapped: Option<String>,
    picking: RwSignal<Option<String>>,
    open_slots: RwSignal<Vec<String>>,
    status: RwSignal<String>,
    reload: impl Fn() + Copy + Send + Sync + 'static,
    in_folder: bool,
) -> AnyView {
    let row_key = format!("{}/{}/{}", item.project, item.kind, item.file);
    let label = if item.title.is_empty() {
        item.file.clone()
    } else {
        item.title.clone()
    };
    let assign_target = if in_folder {
        String::new()
    } else {
        ch_id.get_value()
    };
    let assign_item = item.clone();
    let sched_item = item.clone();
    let row_key_for_pick = row_key.clone();

    view! {
        <div style="border-top: 1px solid rgba(255,255,255,0.06); padding: 10px 0;">
            <div class="row-between">
                <span>
                    {kind_icon(&item.kind)}
                    " "
                    {label}
                    {item.published.then(|| view! { <span class="pill">" published"</span> })}
                </span>
                <span class="row" style="gap:6px">
                    <a class="btn btn-ghost btn-sm" href=item.path.clone() target="_blank">
                        "▶"
                    </a>
                    <button
                        class="btn btn-ghost btn-sm"
                        on:click=move |_| {
                            let body = AssignReq {
                                project: assign_item.project.clone(),
                                kind: assign_item.kind.clone(),
                                file: assign_item.file.clone(),
                                channel: assign_target.clone(),
                            };
                            spawn_local(async move {
                                match post_json::<AssignReq, OkResp>("/api/library/assign", &body)
                                    .await
                                {
                                    Ok(_) => {
                                        status.set("✓ filed".into());
                                        reload();
                                    }
                                    Err(e) => status.set(format!("⚠ assign: {e}")),
                                }
                            });
                        }
                    >
                        {if in_folder { "✕ unfile" } else { "📁 file here" }}
                    </button>
                    {(in_folder && mapped.is_some())
                        .then(|| {
                            let rk = row_key.clone();
                            view! {
                                <button
                                    class="btn btn-primary btn-sm"
                                    on:click=move |_| {
                                        let rk = rk.clone();
                                        let id = ch_id.get_value();
                                        picking.set(Some(rk));
                                        open_slots.set(vec![]);
                                        spawn_local(async move {
                                            match get_json::<
                                                SlotsResp,
                                            >(&format!("/api/channels/{id}/slots-next?n=6"))
                                                .await
                                            {
                                                Ok(r) => open_slots.set(r.slots),
                                                Err(e) => status.set(format!("⚠ slots: {e}")),
                                            }
                                        });
                                    }
                                >
                                    "📅 Deploy"
                                </button>
                            }
                        })}
                </span>
            </div>
            {move || {
                (picking.get().as_deref() == Some(row_key_for_pick.as_str()))
                    .then(|| {
                        let slots = open_slots.get();
                        let mapped_id = mapped.clone().unwrap_or_default();
                        let sched = sched_item.clone();
                        view! {
                            <div class="row mb-8" style="gap:6px; flex-wrap: wrap; margin-top: 8px;">
                                {if slots.is_empty() {
                                    view! { <span class="muted">"finding open slots…"</span> }
                                        .into_any()
                                } else {
                                    slots
                                        .into_iter()
                                        .map(|slot| {
                                            let s = sched.clone();
                                            let iid = mapped_id.clone();
                                            let slot_for_req = slot.clone();
                                            view! {
                                                <button
                                                    class="btn btn-ghost btn-sm"
                                                    on:click=move |_| {
                                                        let title = if s.title.is_empty() {
                                                            s.file.clone()
                                                        } else {
                                                            s.title.clone()
                                                        };
                                                        let body = PublishReq {
                                                            path: s.path.clone(),
                                                            integration_id: iid.clone(),
                                                            title,
                                                            schedule: "schedule".into(),
                                                            date: slot_for_req.clone(),
                                                        };
                                                        picking.set(None);
                                                        status.set("⏳ scheduling…".into());
                                                        spawn_local(async move {
                                                            match post_json::<
                                                                PublishReq,
                                                                serde_json::Value,
                                                            >("/api/postiz/publish", &body)
                                                                .await
                                                            {
                                                                Ok(_) => {
                                                                    status.set("✓ deployed to the queue".into());
                                                                    reload();
                                                                }
                                                                Err(e) => status.set(format!("⚠ deploy: {e}")),
                                                            }
                                                        });
                                                    }
                                                >
                                                    {slot_label(&slot)}
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                                <button
                                    class="btn btn-ghost btn-sm"
                                    on:click=move |_| picking.set(None)
                                >
                                    "cancel"
                                </button>
                            </div>
                        }
                    })
            }}
        </div>
    }
    .into_any()
}
