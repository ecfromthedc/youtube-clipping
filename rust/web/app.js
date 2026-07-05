// Tides & Ships Technology — editor frontend.
// Single-page router: dashboard → project → editor timeline. Talks to the Rust
// pipeline over the small REST API in server.rs. No framework, no build step —
// just modules and the platform. Runs as static files embedded in the binary.

const api = {
  async get(path) {
    const r = await fetch(path);
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },
  async post(path, body) {
    const r = await fetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },
  async upload(path, file) {
    const fd = new FormData();
    fd.append("file", file);
    const r = await fetch(path, { method: "POST", body: fd });
    if (!r.ok) throw new Error((await r.json()).error || r.statusText);
    return r.json();
  },
  async del(path) {
    const r = await fetch(path, { method: "DELETE" });
    if (!r.status === 204) throw new Error(r.statusText);
  },
};

const fmt = {
  time(s) {
    if (!isFinite(s)) return "0:00";
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60).toString().padStart(2, "0");
    return `${m}:${sec}`;
  },
  duration(secs) {
    if (!secs) return "—";
    if (secs < 60) return `${Math.round(secs)}s`;
    const m = Math.floor(secs / 60);
    const s = Math.round(secs % 60);
    return `${m}m ${s}s`;
  },
  date(stamp) {
    try { return new Date(stamp).toLocaleString(); }
    catch { return stamp; }
  },
};

// ── Tiny component helpers ─────────────────────────────────────────────
const h = (tag, props = {}, ...children) => {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") el.className = v;
    else if (k === "html") el.innerHTML = v;
    else if (k.startsWith("on")) el.addEventListener(k.slice(2).toLowerCase(), v);
    else if (v !== null && v !== undefined) el.setAttribute(k, v);
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.appendChild(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
};
const clear = (el) => { while (el.firstChild) el.removeChild(el.firstChild); };

// ── Brand mark (SVG ship + wave) ───────────────────────────────────────
const BRAND_SVG = `<svg viewBox="0 0 64 64" fill="none">
  <path d="M8 42c4 0 4-3 8-3s4 3 8 3 4-3 8-3 4 3 8 3 4-3 8-3 4 3 8 3" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>
  <path d="M32 14l9 13H23z" fill="currentColor"/>
  <rect x="29" y="26" width="6" height="10" rx="1" fill="currentColor"/>
</svg>`;

// ── Topbar ─────────────────────────────────────────────────────────────
function topbar(route) {
  const nav = (label, href) =>
    h("a", { class: `nav-link ${route === href ? "active" : ""}`, href: `#${href}` }, label);
  return h("header", { class: "topbar" },
    h("a", { class: "brand", href: "#/" },
      h("span", { class: "brand-mark", html: BRAND_SVG }),
      h("span", { class: "brand-name" },
        h("span", {}, "Tides"),
        h("span", { class: "amp" }, "&"),
        h("span", {}, "Ships"),
        h("span", { class: "tech" }, "TECHNOLOGY"),
      ),
    ),
    h("nav", { class: "topbar-nav" },
      nav("Projects", "/"),
      nav("Pipeline", "/pipeline"),
      h("span", { class: "pill live" }, h("span", { class: "dot" }), "online"),
    ),
  );
}

// ── Router ─────────────────────────────────────────────────────────────
async function route() {
  const hash = location.hash.replace(/^#/, "") || "/";
  const app = document.getElementById("app");
  clear(app);
  app.appendChild(topbar(hash));

  const page = h("main", { class: "page" });
  app.appendChild(page);

  try {
    if (hash === "/" || hash === "") await dashboardPage(page);
    else if (hash === "/pipeline") await pipelinePage(page);
    else if (hash.startsWith("/p/")) await projectPage(page, hash.slice(3));
    else if (hash.startsWith("/new")) await newProjectPage(page);
    else page.appendChild(h("div", { class: "empty" }, "Not found."));
  } catch (err) {
    page.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
  }
}

window.addEventListener("hashchange", route);

// ── Dashboard ──────────────────────────────────────────────────────────
async function dashboardPage(page) {
  page.appendChild(h("div", { class: "page-header" },
    h("div", {},
      h("h1", { class: "page-title" }, "Projects"),
      h("p", { class: "page-sub" },
        "Drop in raw footage. The pipeline transcribes it, ranks your best moments, and renders a captioned 9:16 clip ready to post.",
      ),
    ),
    h("a", { class: "btn btn-primary", href: "#/new" }, "+ New project"),
  ));

  const grid = h("div", { class: "proj-grid" });
  page.appendChild(grid);
  grid.appendChild(h("div", { class: "empty" }, h("div", { class: "spinner" }), "Loading…"));

  const { projects } = await api.get("/api/projects");
  clear(grid);
  if (projects.length === 0) {
    grid.appendChild(h("div", { class: "panel-soft", style: "grid-column: 1/-1; padding: 48px;" },
      h("div", { class: "empty" },
        h("div", { class: "empty-icon" }, "⛵"),
        h("div", {}, "No projects yet."),
        h("div", { class: "mt-8" },
          h("a", { class: "btn btn-primary btn-sm", href: "#/new" }, "Start your first"),
        ),
      ),
    ));
    return;
  }
  for (const p of projects) {
    grid.appendChild(h("a", {
      class: "proj-card",
      href: `#/p/${p.id}`,
    },
      h("div", { class: "proj-thumb" }, "🎬"),
      h("div", { class: "proj-name" }, p.filename || "untitled"),
      h("div", { class: "proj-meta" },
        h("span", {}, `⏱ ${fmt.duration(p.duration)}`),
        h("span", {}, `✂ ${p.candidates} cands`),
        h("span", {}, `📦 ${p.renders} rendered`),
      ),
    ));
  }
}

// ── Pipeline page (about) ──────────────────────────────────────────────
async function pipelinePage(page) {
  page.appendChild(h("div", { class: "page-header" },
    h("div", {},
      h("h1", { class: "page-title" }, "Pipeline"),
      h("p", { class: "page-sub" },
        "Every step in the chain is a Rust module that already exists in the clipping engine. The editor just orchestrates them for interactive use.",
      ),
    ),
  ));

  const steps = [
    ["01", "Upload", "Source video lands in data/editor/<id>/source.mp4. ffprobe reports duration."],
    ["02", "Transcribe", "whisper.cpp (or openai-whisper fallback) → word-level transcript. Pure-Rust SRT parse."],
    ["03", "Plan clips", "clip::plan_clips groups segments into 15–38s windows, scores each by hook strength."],
    ["04", "Edit", "Pick a window on the timeline. Drag start/end. Set the hook title burned in at top."],
    ["05", "Render", "ffmpeg trims → reframe 9:16 → ab_glyph burns opus-style word-by-word captions."],
    ["06", "Ship", "MP4 lands in data/editor/<id>/renders/. Download or push to a channel."],
  ];

  const grid = h("div", { class: "proj-grid" });
  for (const [n, title, body] of steps) {
    grid.appendChild(h("div", { class: "panel" },
      h("div", { class: "row" },
        h("span", { class: "pill" }, n),
        h("strong", {}, title),
      ),
      h("p", { class: "muted mt-8" }, body),
    ));
  }
  page.appendChild(grid);
}

// ── New project (upload flow) ──────────────────────────────────────────
async function newProjectPage(page) {
  page.appendChild(h("div", { class: "hero" },
    h("div", { class: "hero-badge" }, h("span", { class: "dot" }), "STEP 1 · UPLOAD"),
    h("h1", {}, "Drop the footage."),
    h("p", {},
      "Pick the raw video. Once it's uploaded we'll transcribe it and surface the moments most likely to land as a Short.",
    ),
  ));

  const dz = h("label", { class: "dropzone" },
    h("div", { class: "dropzone-icon" },
      h("svg", { width: "28", height: "28", viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": "2" },
        h("path", { d: "M12 16V4M12 4l-4 4M12 4l4 4" }),
        h("path", { d: "M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2" }),
      ),
    ),
    h("div", { class: "dropzone-title" }, "Click to browse or drop a file"),
    h("div", { class: "dropzone-sub" }, "MP4, MOV, MKV — anything ffmpeg understands"),
    h("input", { type: "file", accept: "video/*", style: "display:none" }),
  );

  const input = dz.querySelector("input");
  const status = h("div", { class: "mt-16" });

  let projectId = null;

  const handleFile = async (file) => {
    try {
      clear(status);
      status.appendChild(h("div", { class: "row" }, h("div", { class: "spinner" }), `Uploading ${file.name}…`));
      const created = await api.post("/api/projects", { filename: file.name });
      projectId = created.id;
      await api.upload(`/api/projects/${projectId}/upload`, file);
      clear(status);
      status.appendChild(h("div", { class: "alert alert-info" },
        `✓ Uploaded. Transcribing now — this takes ~10% of the video length.`,
      ));
      // Auto-advance to project page where transcription runs.
      setTimeout(() => { location.hash = `/p/${projectId}`; }, 600);
    } catch (err) {
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    }
  };

  input.addEventListener("change", (e) => { if (e.target.files[0]) handleFile(e.target.files[0]); });
  dz.addEventListener("dragover", (e) => { e.preventDefault(); dz.classList.add("drag"); });
  dz.addEventListener("dragleave", () => dz.classList.remove("drag"));
  dz.addEventListener("drop", (e) => {
    e.preventDefault();
    dz.classList.remove("drag");
    if (e.dataTransfer.files[0]) handleFile(e.dataTransfer.files[0]);
  });

  page.appendChild(h("div", { class: "panel", style: "max-width: 720px; margin: 0 auto;" }, dz, status));
}

// ── Project page (editor) ──────────────────────────────────────────────
async function projectPage(page, id) {
  const header = h("div", { class: "page-header" });
  page.appendChild(header);

  const editor = h("div", { class: "editor" });
  page.appendChild(editor);

  let project = null;

  const render = () => {
    clear(header);
    clear(editor);

    header.appendChild(h("div", {},
      h("h1", { class: "page-title" }, project.filename || "untitled"),
      h("p", { class: "page-sub" },
        h("span", { class: "mono" }, project.id), " · ",
        h("span", {}, fmt.duration(project.duration)), " · ",
        h("span", {}, `${project.candidates.length} candidates`),
      ),
    ));
    header.appendChild(h("div", { class: "row" },
      h("a", { class: "btn btn-ghost btn-sm", href: "#/" }, "← All projects"),
      h("button", {
        class: "btn btn-danger btn-sm",
        onclick: async () => {
          if (!confirm("Delete this project and all renders?")) return;
          await api.del(`/api/projects/${id}`);
          location.hash = "/";
        },
      }, "Delete"),
    ));

    // LEFT: player + timeline
    const main = h("div", { class: "editor-main" });
    const sidebar = h("div", { class: "editor-sidebar" });
    editor.appendChild(main);
    editor.appendChild(sidebar);

    // Player
    const videoSrc = `/api/projects/${id}/files/source.mp4`;
    const player = h("video", {
      class: "player",
      src: videoSrc,
      controls: true,
      playsinline: true,
    });
    const playerWrap = h("div", { class: "player-wrap" }, player);
    main.appendChild(playerWrap);

    // Timeline
    if (project.candidates.length > 0) {
      main.appendChild(timelineView(project, player));
    } else {
      main.appendChild(h("div", { class: "panel" },
        h("div", { class: "row-between mb-16" },
          h("strong", {}, "Auto-clip timeline"),
          h("span", { class: "muted" }, "Transcribe first"),
        ),
        h("div", { class: "alert alert-info" },
          project.duration > 0
            ? "Click Transcribe to surface ranked clip moments."
            : "No source video yet — drop one in from the dashboard.",
        ),
      ));
    }

    // SIDEBAR
    sidebar.appendChild(sidebarActions(project, player));
    if (project.candidates.length > 0) sidebar.appendChild(candidateList(project, player));
    if (project.renders.length > 0) sidebar.appendChild(rendersList(project));
  };

  // First load
  try {
    project = await api.get(`/api/projects/${id}`);
  } catch (err) {
    page.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    return;
  }
  render();

  // If we have a video but no transcript, surface a one-click transcribe button.
  // (The new-project flow jumps straight here after upload.)
}

// ── Timeline component ─────────────────────────────────────────────────
function timelineView(project, player) {
  const wrap = h("div", { class: "panel" },
    h("div", { class: "row-between mb-8" },
      h("strong", {}, "Auto-clip timeline"),
      h("span", { class: "muted" }, "click a block to scrub"),
    ),
  );

  const dur = project.duration || 1;
  const tl = h("div", { class: "timeline" });

  // ruler
  const ruler = h("div", { class: "timeline-ruler" });
  const ticks = 10;
  for (let i = 0; i < ticks; i++) {
    ruler.appendChild(h("span", {}, fmt.time((dur * i) / ticks)));
  }
  tl.appendChild(ruler);

  // track
  const track = h("div", { class: "timeline-track" });
  tl.appendChild(track);

  const playhead = h("div", { class: "timeline-playhead", style: "left: 0%;" });
  tl.appendChild(playhead);

  for (const c of project.candidates) {
    const left = (c.start / dur) * 100;
    const width = ((c.end - c.start) / dur) * 100;
    const block = h("div", {
      class: "timeline-cand",
      style: `left: ${left}%; width: ${width}%`,
      title: c.text,
      onclick: () => {
        if (player) {
          player.currentTime = c.start;
          player.play();
        }
      },
    },
      h("span", { class: "score" }, c.score.toFixed(2)),
      c.text.slice(0, 40),
    );
    track.appendChild(block);
  }

  // playhead sync
  if (player) {
    player.addEventListener("timeupdate", () => {
      const pct = dur > 0 ? (player.currentTime / dur) * 100 : 0;
      playhead.style.left = `${Math.min(100, Math.max(0, pct))}%`;
    });
  }

  wrap.appendChild(tl);
  return wrap;
}

// ── Sidebar: actions (transcribe, manual clip) ────────────────────────
function sidebarActions(project, player) {
  const section = h("div", { class: "sidebar-section" });
  section.appendChild(h("h3", {}, "Actions"));

  const hasVideo = project.duration > 0;
  const hasTranscript = project.transcript.length > 0;

  if (!hasVideo) {
    section.appendChild(h("div", { class: "alert alert-warn" }, "Upload a source video first."));
    return section;
  }

  if (!hasTranscript) {
    const btn = h("button", { class: "btn btn-primary", style: "width:100%" },
      h("span", { class: "spinner hidden" }),
      "Transcribe & find clips",
    );
    btn.addEventListener("click", async () => {
      btn.disabled = true;
      btn.querySelector(".spinner").classList.remove("hidden");
      btn.lastChild.textContent = " Working…";
      try {
        const updated = await api.post(`/api/projects/${project.id}/transcribe`, {});
        Object.assign(project, updated);
        btn.textContent = "✓ Done — refresh view";
        setTimeout(() => location.reload(), 500);
      } catch (err) {
        btn.disabled = false;
        btn.querySelector(".spinner").classList.add("hidden");
        btn.lastChild.textContent = ` ⚠ ${err.message}`;
      }
    });
    section.appendChild(btn);
    section.appendChild(h("p", { class: "muted mt-8", style: "font-size:12px" },
      "Runs whisper.cpp over the audio. Takes ~10% of the video's length.",
    ));
    return section;
  }

  // Transcribed → manual clip form
  const startIn = h("input", { class: "input", type: "number", step: "0.1", placeholder: "0.0" });
  const endIn = h("input", { class: "input", type: "number", step: "0.1", placeholder: project.duration?.toFixed(1) });
  const titleIn = h("input", { class: "input", placeholder: "Hook title (optional)" });

  // Pre-fill with the top candidate.
  if (project.candidates[0]) {
    startIn.value = project.candidates[0].start.toFixed(1);
    endIn.value = project.candidates[0].end.toFixed(1);
    titleIn.value = project.candidates[0].text.slice(0, 60);
  }

  const renderBtn = h("button", { class: "btn btn-primary", style: "width:100%" }, "Render 9:16 clip");
  const status = h("div", { class: "mt-8" });

  renderBtn.addEventListener("click", async () => {
    const start = parseFloat(startIn.value);
    const end = parseFloat(endIn.value);
    if (!isFinite(start) || !isFinite(end) || end <= start) {
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, "Enter a valid start/end."));
      return;
    }
    renderBtn.disabled = true;
    clear(status);
    status.appendChild(h("div", { class: "row" }, h("div", { class: "spinner" }), "Rendering… trim + reframe + captions."));
    try {
      const out = await api.post(`/api/projects/${project.id}/render`, {
        start, end,
        title: titleIn.value || null,
      });
      clear(status);
      status.appendChild(h("div", { class: "alert alert-info" },
        `✓ Rendered. `,
        h("a", { href: out.path, download: "" }, "Download MP4"),
      ));
      // Refresh the project so the renders list updates.
      const updated = await api.get(`/api/projects/${project.id}`);
      Object.assign(project, updated);
      setTimeout(() => location.reload(), 800);
    } catch (err) {
      renderBtn.disabled = false;
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    }
  });

  section.appendChild(h("div", { class: "field-row" },
    h("div", { class: "field" }, h("label", {}, "Start (s)"), startIn),
    h("div", { class: "field" }, h("label", {}, "End (s)"), endIn),
  ));
  section.appendChild(h("div", { class: "field" }, h("label", {}, "Hook title"), titleIn));
  section.appendChild(renderBtn);
  section.appendChild(status);

  return section;
}

// ── Sidebar: candidate list ───────────────────────────────────────────
function candidateList(project, player) {
  const section = h("div", { class: "sidebar-section" });
  section.appendChild(h("h3", {}, `Ranked moments (${project.candidates.length})`));
  const list = h("div", { class: "cand-list" });

  // Sort a copy best-first (already best-first from plan_clips, but be safe).
  const sorted = [...project.candidates].sort((a, b) => b.score - a.score);

  for (const c of sorted) {
    const hot = c.score >= 3.0;
    const card = h("div", { class: "cand-item", title: c.text, onclick: () => {
      if (player) { player.currentTime = c.start; player.play(); }
    }},
      h("div", { class: "cand-head" },
        h("span", { class: `cand-score ${hot ? "hot" : ""}` }, `★ ${c.score.toFixed(2)}`),
        h("span", { class: "cand-time" }, `${fmt.time(c.start)}–${fmt.time(c.end)} · ${c.duration.toFixed(1)}s`),
      ),
      h("div", { class: "cand-text" }, c.text),
    );
    list.appendChild(card);
  }

  section.appendChild(list);
  return section;
}

// ── Sidebar: renders ───────────────────────────────────────────────────
function rendersList(project) {
  const section = h("div", { class: "sidebar-section" });
  section.appendChild(h("h3", {}, `Renders (${project.renders.length})`));
  const list = h("div", { class: "renders" });
  for (const r of project.renders) {
    list.appendChild(h("div", { class: "render-card" },
      h("div", { class: "render-thumb" }, "📦"),
      h("div", { class: "render-info" },
        h("div", { class: "render-title" }, r.title || "untitled"),
        h("div", { class: "render-meta" }, r.path.split("/").pop()),
      ),
      h("a", { class: "btn btn-ghost btn-sm", href: `/api/projects/${project.id}/files/${r.path}`, download: "" }, "↓"),
    ));
  }
  section.appendChild(list);
  return section;
}

// ── Boot ───────────────────────────────────────────────────────────────
route();
