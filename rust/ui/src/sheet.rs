//! Bottom sheet — the phone-native action surface (P6 gesture polish).
//!
//! Ported from the campaign hub's proven Phase-6 sheet: dismiss via backdrop
//! tap, ✕, Escape, or dragging the handle down past a threshold. The sheet
//! stays mounted and toggles `display` so form state survives close/reopen
//! and the entrance animation replays.

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Drag distance (px) past which release commits a dismiss.
const DRAG_DISMISS_PX: f64 = 100.0;

#[component]
pub fn Sheet(
    open: RwSignal<bool>,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    // Escape closes the sheet; harmless no-op when closed.
    let esc = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || esc.remove());

    // Drag-to-dismiss: track finger offset from the handle; commit past
    // DRAG_DISMISS_PX, else snap back (CSS transition on `.rt-sheet`).
    let drag_y = RwSignal::new(0.0f64);
    let dragging = RwSignal::new(false);
    let start_y = RwSignal::new(0.0f64);

    let on_down = move |e: ev::PointerEvent| {
        start_y.set(e.client_y() as f64);
        dragging.set(true);
        // Capture so move/up keep firing even if the finger leaves the handle.
        if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = el.set_pointer_capture(e.pointer_id());
        }
    };
    let on_move = move |e: ev::PointerEvent| {
        if dragging.get_untracked() {
            drag_y.set((e.client_y() as f64 - start_y.get_untracked()).max(0.0));
        }
    };
    let on_up = move |_: ev::PointerEvent| {
        if !dragging.get_untracked() {
            return;
        }
        dragging.set(false);
        if drag_y.get_untracked() > DRAG_DISMISS_PX {
            open.set(false);
        }
        drag_y.set(0.0);
    };

    let sheet_class = move || {
        if dragging.get() {
            "rt-sheet rt-dragging"
        } else {
            "rt-sheet"
        }
    };
    // No inline transform at rest → the entrance keyframe runs untouched.
    let sheet_style = move || {
        let y = drag_y.get();
        if y == 0.0 {
            String::new()
        } else {
            format!("transform:translateY({y}px)")
        }
    };
    let wrap_style = move || {
        if open.get() {
            "display:contents"
        } else {
            "display:none"
        }
    };

    view! {
        <div class="rt-monly" style=wrap_style>
            <div class="rt-sheet-backdrop" on:click=move |_| open.set(false)></div>
            <div class=sheet_class style=sheet_style>
                <div
                    class="rt-sheet-draghandle"
                    on:pointerdown=on_down
                    on:pointermove=on_move
                    on:pointerup=on_up
                    on:pointercancel=on_up
                >
                    <div class="rt-sheet-handle"></div>
                </div>
                <div class="rt-sheet-head">
                    <span class="title">{title}</span>
                    <button type="button" class="rt-sheet-x" on:click=move |_| open.set(false)>
                        "✕"
                    </button>
                </div>
                <div class="rt-sheet-body">{children()}</div>
            </div>
        </div>
    }
}
