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
      nav("Studio", "/studio"),
      nav("Analytics", "/analytics"),
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
    else if (hash === "/studio") await studioPage(page);
    else if (hash.startsWith("/studio/")) await studioFormatPage(page, hash.slice(8));
    else if (hash === "/analytics") await analyticsPage(page);
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
    if (project.candidates.length >= 2) sidebar.appendChild(compileSection(project));
    if (project.candidates.length > 0) sidebar.appendChild(candidateList(project, player));
    if (project.renders.length > 0) sidebar.appendChild(rendersList(project));
    if (project.compiles && project.compiles.length > 0) sidebar.appendChild(compilesList(project));
    if (project.stories && project.stories.length > 0) sidebar.appendChild(storiesList(project));
    if (project.commentary && project.commentary.length > 0) sidebar.appendChild(commentaryList(project));
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
      renderCardActions(project.id, r.path, r.title),
    ));
  }
  section.appendChild(list);
  return section;
}

// ── Sidebar: storytelling + commentary ────────────────────────────────
function _studioOutputList(project, items, title, icon, badgeColor) {
  const section = h("div", { class: "sidebar-section" });
  section.appendChild(h("h3", {}, `${title} (${items.length})`));
  const list = h("div", { class: "renders" });
  for (const r of items) {
    list.appendChild(h("div", { class: "render-card" },
      h("div", { class: "render-thumb", style: `background: ${badgeColor}; color: white;` }, icon),
      h("div", { class: "render-info" },
        h("div", { class: "render-title" }, r.title || "untitled"),
        h("div", { class: "render-meta" }, r.path.split("/").pop()),
      ),
      renderCardActions(project.id, r.path, r.title),
    ));
  }
  section.appendChild(list);
  return section;
}
const storiesList = (project) => _studioOutputList(project, project.stories, "Storytelling", "📖", "linear-gradient(135deg,#7c3aed,#5b21b6)");
const commentaryList = (project) => _studioOutputList(project, project.commentary, "Commentary", "🎬", "linear-gradient(135deg,#0891b2,#155e75)");

// ── Sidebar: compiles (ranking listicles) ──────────────────────────────
function compilesList(project) {
  const section = h("div", { class: "sidebar-section" });
  section.appendChild(h("h3", {}, `Compilations (${project.compiles.length})`));
  const list = h("div", { class: "renders" });
  for (const r of project.compiles) {
    list.appendChild(h("div", { class: "render-card" },
      h("div", { class: "render-thumb", style: "background: var(--brand-gradient); color: white;" }, "🏆"),
      h("div", { class: "render-info" },
        h("div", { class: "render-title" }, r.title || "ranking compilation"),
        h("div", { class: "render-meta" }, r.path.split("/").pop()),
      ),
      renderCardActions(project.id, r.path, r.title),
    ));
  }
  section.appendChild(list);
  return section;
}

// ── Sidebar: ranking compile builder ───────────────────────────────────
function compileSection(project) {
  const section = h("div", { class: "sidebar-section compile-section" });
  section.appendChild(h("h3", {}, "🏆 Ranking compilation"));

  // Sortable list of picks. Start from top-N candidates, best LAST (countup reveal —
  // the reference reel saves the best moment for last).
  const initialPicks = [...project.candidates]
    .sort((a, b) => a.score - b.score) // worst → best; first played, last is the punchline
    .slice(0, Math.min(5, project.candidates.length));
  let picks = initialPicks.map((c) => ({ start: c.start, end: c.end, label: c.text.slice(0, 60) }));

  const titleInput = h("input", {
    class: "input",
    placeholder: "Hook title (e.g. 'Top 5 Funniest Moments')",
    value: project.candidates[0]?.text?.slice(0, 60) || "",
  });

  // Order toggle — countup (default, best last) vs countdown.
  let order = "countup";
  const orderToggle = h("div", { class: "compile-order-toggle" });
  const upBtn = h("button", { class: "compile-order-btn active" }, "▲ Best last");
  const downBtn = h("button", { class: "compile-order-btn" }, "▼ Best first");
  const setOrder = (val, active, inactive) => {
    order = val;
    active.classList.add("active");
    inactive.classList.remove("active");
  };
  upBtn.addEventListener("click", () => setOrder("countup", upBtn, downBtn));
  downBtn.addEventListener("click", () => setOrder("countdown", downBtn, upBtn));
  orderToggle.appendChild(upBtn);
  orderToggle.appendChild(downBtn);

  const list = h("div", { class: "compile-list" });

  const renderList = () => {
    clear(list);
    picks.forEach((pick, i) => {
      const card = h("div", { class: "compile-item" },
        h("div", { class: "compile-rank", style: i === picks.length - 1 ? "background: var(--brand-gradient);" : "" }, String(i + 1)),
        h("div", { class: "compile-pick-text" }, pick.label || `${pick.start.toFixed(1)}–${pick.end.toFixed(1)}s`),
        h("div", { class: "compile-controls" },
          h("button", {
            class: "btn btn-ghost btn-sm",
            title: "Move up",
            onclick: () => {
              if (i === 0) return;
              [picks[i - 1], picks[i]] = [picks[i], picks[i - 1]];
              renderList();
            },
          }, "↑"),
          h("button", {
            class: "btn btn-ghost btn-sm",
            title: "Move down",
            onclick: () => {
              if (i === picks.length - 1) return;
              [picks[i + 1], picks[i]] = [picks[i], picks[i + 1]];
              renderList();
            },
          }, "↓"),
          h("button", {
            class: "btn btn-danger btn-sm",
            title: "Remove",
            onclick: () => {
              picks.splice(i, 1);
              renderList();
            },
          }, "✕"),
        ),
      );
      list.appendChild(card);
    });
    if (picks.length < 2) {
      list.appendChild(h("div", { class: "muted mt-8", style: "font-size: 12px;" }, "Need at least 2 picks to compile."));
    }
  };
  renderList();

  const compileBtn = h("button", { class: "btn btn-primary", style: "width: 100%;" }, "Compile ranking video");
  const status = h("div", { class: "mt-8" });

  compileBtn.addEventListener("click", async () => {
    if (picks.length < 2) {
      clear(status);
      status.appendChild(h("div", { class: "alert alert-warn" }, "Add at least 2 picks to compile."));
      return;
    }
    compileBtn.disabled = true;
    clear(status);
    status.appendChild(h("div", { class: "row" },
      h("div", { class: "spinner" }),
      `Compiling ${picks.length} clips → one ranking video…`,
    ));
    try {
      const out = await api.post(`/api/projects/${project.id}/compile`, {
        items: picks.map((p) => ({ start: p.start, end: p.end, label: p.label })),
        title: titleInput.value || null,
        order,
      });
      clear(status);
      status.appendChild(h("div", { class: "alert alert-info" },
        `✓ Compiled ${out.segments} clips (${out.duration.toFixed(1)}s). `,
        h("a", { href: out.path, download: "" }, "Download MP4"),
      ));
      setTimeout(() => location.reload(), 1000);
    } catch (err) {
      compileBtn.disabled = false;
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    }
  });

  section.appendChild(h("p", { class: "muted", style: "font-size: 12px; margin: 0 0 10px;" },
    "Cuts each ranked moment, stamps a rank number on the left edge, concatenates into one 9:16 video.",
  ));
  section.append(
    titleInput,
    h("div", { class: "field", style: "margin-top: 10px;" }, h("label", {}, "Reveal order"), orderToggle),
    list,
    h("div", { style: "margin-top: 12px;" }, compileBtn),
    status,
  );
  return section;
}

// ── Studio: format picker ──────────────────────────────────────────────
const FORMATS = [
  {
    slug: "ranking",
    name: "Ranking Compilation",
    icon: "🏆",
    blurb: "Top-N ranked clips, big rank numbers on the left edge, countdown reveal (best plays last). The highest-volume format.",
    difficulty: "Easy",
    href: "#/p/",
    cta: "Open a project →",
    note: "Lives inside each project (after upload + transcribe)",
  },
  {
    slug: "storytelling",
    name: "Storytelling / Roblox Rants",
    icon: "📖",
    blurb: "Write a script → AI voiceover → looping gameplay background → opus captions → 9:16. Generates original content from words.",
    difficulty: "Easy",
    href: "#/studio/storytelling",
    cta: "Write a script",
    note: "Needs OmniVoice + a background clip",
  },
  {
    slug: "commentary",
    name: "Commentary / Reaction",
    icon: "🎬",
    blurb: "Paste a viral clip → write commentary → AI VO over ducked original audio + captions. His highest-RPM niche (35-40¢/1k).",
    difficulty: "Medium",
    href: "#/studio/commentary",
    cta: "React to a clip",
    note: "Needs OmniVoice + a source clip",
  },
];

async function studioPage(page) {
  page.appendChild(h("div", { class: "page-header" },
    h("div", {},
      h("h1", { class: "page-title" }, "Studio"),
      h("p", { class: "page-sub" },
        "Three formats, one engine. Each is an end-to-end play from the playbook — pick one and ship a Short.",
      ),
    ),
  ));

  // OmniVoice status banner
  const vo = await api.get("/api/voices").catch(() => ({ available: false, voices: [] }));
  if (!vo.available) {
    page.appendChild(h("div", { class: "alert alert-warn mb-24" },
      "⚠ OmniVoice Studio isn't reachable at localhost:3900. Storytelling + Commentary need it for voiceover. ",
      h("a", { href: "#/pipeline" }, "Start it →"),
    ));
  }

  const grid = h("div", { class: "format-grid" });
  for (const f of FORMATS) {
    grid.appendChild(h("a", { class: "format-card", href: f.href },
      h("div", { class: "format-icon" }, f.icon),
      h("div", { class: "format-name" }, f.name),
      h("div", { class: "format-blurb" }, f.blurb),
      h("div", { class: "format-meta" },
        h("span", { class: `pill ${f.difficulty === "Easy" ? "" : "pill-warn"}` }, f.difficulty),
        h("span", { class: "muted", style: "font-size:11px;" }, f.note),
      ),
      h("div", { class: "format-cta" }, f.cta, " →"),
    ));
  }
  page.appendChild(grid);
}

async function studioFormatPage(page, slug) {
  const format = FORMATS.find((f) => f.slug === slug);
  if (!format || (slug !== "storytelling" && slug !== "commentary")) {
    page.appendChild(h("div", { class: "empty" }, "Unknown format."));
    return;
  }
  // The Rust enum tags on "story" (the variant name lowercased); map from URL slug.
  const formatTag = slug === "storytelling" ? "story" : slug;
  page.appendChild(h("div", { class: "page-header" },
    h("div", {},
      h("a", { class: "muted", href: "#/studio", style: "font-size:13px;" }, "← Studio"),
      h("h1", { class: "page-title mt-8" }, `${format.icon} ${format.name}`),
      h("p", { class: "page-sub" }, format.blurb),
    ),
  ));

  // Voice picker
  const vo = await api.get("/api/voices").catch(() => ({ available: false, voices: [] }));
  const voiceSelect = h("select", { class: "select" },
    h("option", { value: "default" }, "Default voice"),
    ...(vo.voices || []).map((v) =>
      h("option", { value: v.id }, `${v.name} (${v.id})`)
    ),
  );
  if (!vo.available) {
    voiceSelect.disabled = true;
  }

  const titleInput = h("input", { class: "input", placeholder: "Hook title (top of frame, optional)" });
  const scriptInput = h("textarea", {
    class: "textarea",
    style: "min-height: 160px;",
    placeholder: slug === "storytelling"
      ? "Write the story / hot take the VO will read. e.g. 'This is the most ridiculous thing that happened at school today...'"
      : "Write the commentary the VO speaks over the clip. e.g. 'Okay watch what this guy does next — this is actually insane...'",
  });
  const speedInput = h("input", { class: "input", type: "number", step: "0.05", min: "0.5", max: "2.0", value: "1.0", placeholder: "1.0" });
  const langInput = h("input", { class: "input", placeholder: "en (optional)" });

  // Format-specific source field
  let sourceInput, sourceLabel, duckInput = null;
  if (slug === "storytelling") {
    sourceLabel = "Background footage (gameplay/Minecraft) — URL or local path";
    sourceInput = h("input", {
      class: "input",
      placeholder: "https://www.youtube.com/watch?v=... or /path/to/subway_surfer.mp4",
    });
  } else {
    sourceLabel = "Source clip — URL (YT/TT/IG) or local path";
    sourceInput = h("input", {
      class: "input",
      placeholder: "https://www.tiktok.com/@.../video/... or /path/to/clip.mp4",
    });
    duckInput = h("input", { class: "input", type: "number", step: "0.05", min: "0", max: "1", value: "0.25" });
  }

  const renderBtn = h("button", { class: "btn btn-primary", style: "width: 100%;", disabled: !vo.available },
    vo.available ? "Render Short" : "OmniVoice offline — start it first",
  );
  const status = h("div", { class: "mt-16" });

  renderBtn.addEventListener("click", async () => {
    if (!scriptInput.value.trim()) {
      status.appendChild(h("div", { class: "alert alert-warn" }, "Write the script first."));
      return;
    }
    if (!sourceInput.value.trim()) {
      status.appendChild(h("div", { class: "alert alert-warn" }, `Add the ${slug === "storytelling" ? "background" : "source clip"}.`));
      return;
    }
    renderBtn.disabled = true;
    clear(status);
    status.appendChild(h("div", { class: "row" },
      h("div", { class: "spinner" }),
      slug === "storytelling"
        ? "Synthesizing VO → transcribing → compositing gameplay… (60-120s)"
        : "Fetching source → VO → captions → mix… (60-120s)",
    ));
    const body = {
      format: formatTag,
      script: scriptInput.value,
      voice: voiceSelect.value,
      title: titleInput.value || null,
      speed: parseFloat(speedInput.value) || null,
      language: langInput.value || null,
      [slug === "storytelling" ? "background" : "source"]: sourceInput.value,
      ...(duckInput ? { duck_volume: parseFloat(duckInput.value) || 0.25 } : {}),
    };
    try {
      const out = await api.post("/api/studio/render", body);
      clear(status);
      const dl = h("a", { class: "btn btn-primary btn-sm", href: out.path, download: "" }, "↓ Download MP4");
      dl.addEventListener("click", () => setTimeout(() => location.reload(), 800));
      status.appendChild(h("div", { class: "alert alert-info" },
        "✓ Rendered. ", dl,
      ));
    } catch (err) {
      renderBtn.disabled = false;
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    }
  });

  page.appendChild(h("div", { class: "panel", style: "max-width: 760px;" },
    h("div", { class: "field" }, h("label", {}, sourceLabel), sourceInput),
    h("div", { class: "field" }, h("label", {}, "Script (will be spoken by the VO)"), scriptInput),
    h("div", { class: "field" }, h("label", {}, "Hook title"), titleInput),
    h("div", { class: "field-row" },
      h("div", { class: "field" }, h("label", {}, "Voice"), voiceSelect),
      h("div", { class: "field" }, h("label", {}, "Speed"), speedInput),
      h("div", { class: "field" }, h("label", {}, "Language"), langInput),
    ),
    duckInput && h("div", { class: "field" }, h("label", {}, "Original clip duck volume (0-1)"), duckInput),
    renderBtn,
    status,
  ));
}

// ── Analytics dashboard — closes the render → publish → MEASURE → tune loop ──
async function analyticsPage(page) {
  page.appendChild(h("div", { class: "page-header" },
    h("div", {},
      h("h1", { class: "page-title" }, "Analytics"),
      h("p", { class: "page-sub" },
        "Channel rollup, top posts, and the 'what's working' recommendations derived from your own data.",
      ),
    ),
    h("div", { class: "row" },
      h("button", { class: "btn btn-ghost btn-sm", onclick: () => { fetch("/api/analytics/rollup", { method: "GET" }).then(() => location.reload()); } }, "↻ Refresh (1h cache)"),
    ),
  ));

  const loading = h("div", { class: "row mb-24" }, h("div", { class: "spinner" }), "Loading analytics…");
  page.appendChild(loading);

  // Fire all requests in parallel.
  const [rollup, top, daily, recs] = await Promise.all([
    api.get("/api/analytics/rollup?days=28").catch(() => null),
    api.get("/api/analytics/top?days=28&limit=15").catch(() => null),
    api.get("/api/analytics/daily?days=7").catch(() => null),
    api.get("/api/analytics/recommendations").catch(() => null),
  ]);
  loading.remove();

  if (!rollup || !rollup.configured) {
    page.appendChild(h("div", { class: "alert alert-warn" },
      "⚠ YouTube Analytics OAuth not connected. Run ",
      h("code", { class: "mono" }, ".venv/bin/python scripts/yt_oauth.py"),
      " to enable channel rollups + per-video retention.",
    ));
    return;
  }

  // Rollup tiles
  const tiles = [
    { label: "Views (28d)", value: fmt.int(rollup.views || 0), accent: false },
    { label: "Est. Revenue", value: fmt.money(rollup.est_revenue || 0), accent: true },
    { label: "Subs Gained", value: fmt.int(rollup.subs_gained || 0), accent: false },
    { label: "Avg Watch %", value: `${(rollup.avg_watch_pct || 0).toFixed(1)}%`, accent: false },
  ];
  const tilesRow = h("div", { class: "an-tiles" });
  for (const t of tiles) {
    tilesRow.appendChild(h("div", { class: `an-tile ${t.accent ? "an-tile-accent" : ""}` },
      h("div", { class: "an-tile-value" }, t.value),
      h("div", { class: "an-tile-label" }, t.label),
    ));
  }
  page.appendChild(tilesRow);

  // Daily sparkline
  if (daily && (daily.views || []).length > 0) {
    page.appendChild(h("div", { class: "panel mt-16" },
      h("div", { class: "row-between mb-8" },
        h("strong", {}, "Last 7 days"),
        h("span", { class: "muted mono", style: "font-size:11px;" },
          `${fmt.int((daily.views || []).reduce((a, b) => a + b, 0))} views · ${fmt.money((daily.revenue || []).reduce((a, b) => a + b, 0))} est. rev`,
        ),
      ),
      sparkline(daily.views || [], daily.revenue || [], daily.dates || []),
    ));
  }

  // Recommendations
  if (recs && recs.ready) {
    const recPanel = h("div", { class: "panel mt-16" },
      h("h3", { style: "margin: 0 0 12px;" }, "🎯 What's working"),
    );
    for (const r of (recs.recommendations || [])) {
      recPanel.appendChild(h("div", { class: "alert alert-info mb-8" }, `→ ${r}`));
    }
    if (recs.format_breakdown && recs.format_breakdown.length > 0) {
      recPanel.appendChild(h("div", { class: "mt-16" },
        h("strong", { style: "font-size:13px;" }, "Format breakdown"),
        h("table", { class: "an-table mt-8" },
          h("thead", {}, h("tr", {},
            h("th", {}, "Format"), h("th", {}, "Posts"), h("th", {}, "Total views"), h("th", {}, "Revenue"), h("th", {}, "Avg views/post"),
          )),
          h("tbody", {},
            ...recs.format_breakdown.map((f) => h("tr", {},
              h("td", {}, f.format || "—"),
              h("td", { class: "mono" }, String(f.count)),
              h("td", { class: "mono" }, fmt.int(f.total_views)),
              h("td", { class: "mono" }, fmt.money(f.revenue)),
              h("td", { class: "mono" }, fmt.int(f.avg_views)),
            )),
          ),
        ),
      ));
    }
    page.appendChild(recPanel);
  } else if (recs && !recs.ready) {
    page.appendChild(h("div", { class: "panel mt-16" },
      h("strong", {}, "🎯 What's working"),
      h("p", { class: "muted mt-8" }, recs.note || "Not enough posted videos yet."),
    ));
  }

  // Top videos table
  if (top && (top.videos || []).length > 0) {
    page.appendChild(h("div", { class: "panel mt-16" },
      h("h3", { style: "margin: 0 0 12px;" }, `Top ${top.videos.length} videos (28d)`),
      h("table", { class: "an-table" },
        h("thead", {}, h("tr", {},
          h("th", {}, ""),  // health dot
          h("th", {}, "Views"),
          h("th", {}, "Avg watch %"),
          h("th", {}, "Subs"),
          h("th", {}, "Est. rev"),
          h("th", {}, "Video"),
        )),
        h("tbody", {},
          ...top.videos.map((v) => h("tr", {},
            h("td", {}, h("span", { class: `health-dot health-${v.health}` })),
            h("td", { class: "mono" }, fmt.int(v.views || 0)),
            h("td", { class: "mono" }, `${(v.averagePercentageWatched || 0).toFixed(1)}%`),
            h("td", { class: "mono" }, fmt.int(v.subscribersGained || 0)),
            h("td", { class: "mono" }, fmt.money(v.estimatedRevenue || 0)),
            h("td", {}, v.url
              ? h("a", { href: v.url, target: "_blank" }, v.video || "watch ↗")
              : h("span", { class: "mono muted" }, v.video || "—"),
            ),
          )),
        ),
      ),
    ));
  } else {
    page.appendChild(h("div", { class: "panel mt-16" },
      h("strong", {}, "Top videos"),
      h("p", { class: "muted mt-8" }, "No videos with view data in the last 28 days."),
    ));
  }
}

// Sparkline = SVG polyline scaled to fit.
function sparkline(views, revenue, dates) {
  const W = 600, H = 80, PAD = 4;
  if (views.length < 2) return h("div", { class: "muted" }, "Not enough data points.");
  const max = Math.max(...views, 1);
  const stepX = (W - 2 * PAD) / (views.length - 1);
  const pts = views.map((v, i) => `${PAD + i * stepX},${H - PAD - (v / max) * (H - 2 * PAD)}`).join(" ");
  const svg = `<svg viewBox="0 0 ${W} ${H}" class="an-spark">
    <defs><linearGradient id="sg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#e100c3" stop-opacity="0.4"/>
      <stop offset="100%" stop-color="#e100c3" stop-opacity="0"/>
    </linearGradient></defs>
    <polyline points="${pts}" fill="none" stroke="#e100c3" stroke-width="2"/>
    <polygon points="${PAD},${H-PAD} ${pts} ${W-PAD},${H-PAD}" fill="url(#sg)"/>
  </svg>`;
  return h("div", { html: svg });
}

// Format helpers (extend the existing fmt object).
fmt.int = (n) => Number(n || 0).toLocaleString();
fmt.money = (n) => {
  if (!n) return "$0";
  if (n < 100) return `$${n.toFixed(2)}`;
  return `$${Math.round(n).toLocaleString()}`;
};

// ── Publish modal — Postiz publishing from any rendered card ───────────
async function openPublishModal(projectId, renderPath, suggestedTitle) {
  // Fetch integrations
  const root = document.createElement("div");
  root.className = "modal-backdrop";
  const modal = h("div", { class: "modal", style: "max-width: 540px;" },
    h("h3", {}, "📤 Publish to Postiz"),
    h("p", { class: "muted", style: "font-size: 13px; margin: 0 0 16px;" },
      "Uploads the MP4 + creates a post on the chosen YouTube channel via Postiz.",
    ),
  );
  root.appendChild(modal);
  document.body.appendChild(root);
  root.addEventListener("click", (e) => { if (e.target === root) root.remove(); });

  modal.appendChild(h("div", { class: "row" }, h("div", { class: "spinner" }), "Loading channels…"));
  const data = await api.get("/api/postiz/integrations").catch(() => null);
  // Clear spinner
  while (modal.children.length > 2) modal.removeChild(modal.lastChild);

  if (!data || !data.available) {
    modal.appendChild(h("div", { class: "alert alert-error" },
      !data || data.token_configured === false
        ? "POSTIZ_API_TOKEN not configured. Add it to .env."
        : "Couldn't reach Postiz. Check the token + network.",
    ));
    modal.appendChild(h("div", { class: "modal-actions" },
      h("button", { class: "btn", onclick: () => root.remove() }, "Close"),
    ));
    return;
  }

  const integrations = (data.integrations || []).filter((i) => i.identifier === "youtube" && !i.disabled);
  const chanSelect = h("select", { class: "select" },
    ...integrations.map((i) => h("option", { value: i.id }, `${i.name} (@${(i.profile || "").replace(/^@/, "")})`)),
  );
  const titleInput = h("input", { class: "input", value: suggestedTitle || "", placeholder: "YouTube title (≤100 chars)" });
  const captionInput = h("textarea", { class: "textarea", style: "min-height: 60px;", placeholder: "Description (default: title + #shorts)" });
  const scheduleToggle = h("div", { class: "compile-order-toggle", style: "margin-top: 4px;" });
  let schedMode = "now";
  const nowBtn = h("button", { class: "compile-order-btn active" }, "⚡ Post now");
  const schedBtn = h("button", { class: "compile-order-btn" }, "📅 Schedule");
  nowBtn.addEventListener("click", () => { schedMode = "now"; nowBtn.classList.add("active"); schedBtn.classList.remove("active"); });
  schedBtn.addEventListener("click", () => { schedMode = "schedule"; schedBtn.classList.add("active"); nowBtn.classList.remove("active"); });
  scheduleToggle.appendChild(nowBtn);
  scheduleToggle.appendChild(schedBtn);

  const publishBtn = h("button", { class: "btn btn-primary", style: "width: 100%; margin-top: 12px;" }, "📤 Publish");
  const status = h("div", { class: "mt-8" });

  publishBtn.addEventListener("click", async () => {
    if (!titleInput.value.trim()) {
      status.appendChild(h("div", { class: "alert alert-warn" }, "Add a title first."));
      return;
    }
    publishBtn.disabled = true;
    clear(status);
    status.appendChild(h("div", { class: "row" },
      h("div", { class: "spinner" }),
      "Uploading to Postiz + creating post… (30-90s)",
    ));
    try {
      const out = await api.post("/api/postiz/publish", {
        path: renderPath,
        integration_id: chanSelect.value,
        title: titleInput.value,
        caption: captionInput.value || null,
        schedule: schedMode,
      });
      clear(status);
      status.appendChild(h("div", { class: "alert alert-info" },
        `✓ Published. Postiz post id: `, h("span", { class: "mono" }, out.post_id),
      ));
      publishBtn.textContent = "✓ Done";
    } catch (err) {
      publishBtn.disabled = false;
      clear(status);
      status.appendChild(h("div", { class: "alert alert-error" }, `⚠ ${err.message}`));
    }
  });

  modal.append(
    h("div", { class: "field" }, h("label", {}, "Channel"), chanSelect),
    h("div", { class: "field" }, h("label", {}, "Title"), titleInput),
    h("div", { class: "field" }, h("label", {}, "Description (optional)"), captionInput),
    h("div", { class: "field" }, h("label", {}, "When"), scheduleToggle),
    publishBtn,
    status,
    h("div", { class: "modal-actions" },
      h("button", { class: "btn btn-ghost", onclick: () => root.remove() }, "Close"),
    ),
  );
}

// Wrap a render card's existing download button + add a publish button.
function renderCardActions(projectId, renderPath, title) {
  return h("div", { class: "row" },
    h("a", {
      class: "btn btn-ghost btn-sm",
      href: `/api/projects/${projectId}/files/${renderPath}`,
      download: "",
      title: "Download",
    }, "↓"),
    h("button", {
      class: "btn btn-primary btn-sm",
      title: "Publish to Postiz",
      onclick: () => openPublishModal(projectId, `/api/projects/${projectId}/files/${renderPath}`, title),
    }, "📤"),
  );
}

// ── Boot ───────────────────────────────────────────────────────────────
route();
