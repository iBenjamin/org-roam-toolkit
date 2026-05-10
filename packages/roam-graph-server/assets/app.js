// app.js — ortk-roam-graph SPA glue.
// Plain ES module. No bundler. Sigma + graphology load as UMD globals.

const $ = (sel) => document.querySelector(sel);

const state = {
  graph: null,            // graphology.Graph
  sigma: null,            // Sigma instance
  selectedId: null,       // string | null
  activeTags: new Set(),  // Set<string>
  orphansOnly: false,
  allTags: [],            // string[]
};

function readUrlState() {
  const p = new URLSearchParams(location.search);
  state.selectedId = p.get('node');
  state.activeTags = new Set(
    (p.get('tags') || '').split(',').filter(Boolean),
  );
  state.orphansOnly = p.get('orphans') === '1';
}

function writeUrlState() {
  const p = new URLSearchParams();
  if (state.selectedId) p.set('node', state.selectedId);
  if (state.activeTags.size) p.set('tags', [...state.activeTags].join(','));
  if (state.orphansOnly) p.set('orphans', '1');
  history.replaceState(null, '', `?${p.toString()}`);
}

async function fetchGraph() {
  const res = await fetch('/api/graph');
  if (!res.ok) throw new Error(`graph: ${res.status}`);
  return res.json();
}

function buildGraphology(payload) {
  const g = new graphology.Graph({ multi: false, type: 'undirected' });
  for (const n of payload.nodes) {
    g.addNode(n.id, {
      label: n.title,
      size: 4 + Math.min(n.degree, 12),
      color: n.orphan ? '#f5a623' : '#5a8dee',
      x: Math.random(),
      y: Math.random(),
      tags: n.tags,
      orphan: n.orphan,
    });
  }
  for (const e of payload.edges) {
    if (g.hasNode(e.source) && g.hasNode(e.dest) && !g.hasEdge(e.source, e.dest)) {
      g.addEdge(e.source, e.dest, { color: '#3a3f4d', size: 0.5 });
    }
  }
  return g;
}

function startLayout(g) {
  // graphology-layout-forceatlas2@0.10.1 synchronous IIFE.
  // fa2.worker.js registers window.forceAtlas2 = synchronousLayout.
  // Sync tradeoff: blocks the main thread for ~200 iterations but is simpler
  // than a Worker (no postMessage plumbing, no race with Sigma init).
  const fa2 = window.forceAtlas2;
  if (!fa2) {
    console.warn('forceAtlas2 global not found; graph will use random positions');
    return;
  }
  const settings = { gravity: 1, scalingRatio: 8, slowDown: 5 };
  fa2.assign(g, { iterations: 200, settings });
}

function renderSigma(g) {
  state.sigma = new Sigma(g, $('#sigma'), {
    renderEdgeLabels: false,
    labelSize: 10,
    labelDensity: 0.07,
  });
  state.sigma.on('clickNode', ({ node }) => {
    selectNode(node);
  });
  state.sigma.on('clickStage', () => {
    selectNode(null);
  });
}

async function fetchNode(id) {
  const res = await fetch(`/api/node/${encodeURIComponent(id)}`);
  if (!res.ok) throw new Error(`node ${id}: ${res.status}`);
  return res.json();
}

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function renderNotePane(n) {
  const tags = n.tags.map((t) => `<span class="tag-chip">${escapeHtml(t)}</span>`).join(' ');
  const aliases = n.aliases.length
    ? `<p class="panel-meta">aliases: ${escapeHtml(n.aliases.join(', '))}</p>`
    : '';
  const bl = n.backlinks
    .map(
      (b) =>
        `<li data-id="${escapeHtml(b.id)}" class="bl-item">${escapeHtml(b.title)}</li>`,
    )
    .join('');
  const fw = n.forward
    .map(
      (b) =>
        `<li data-id="${escapeHtml(b.id)}" class="bl-item">${escapeHtml(b.title)}</li>`,
    )
    .join('');
  $('#note').innerHTML = `
    <h2>${escapeHtml(n.title)}</h2>
    <p class="panel-meta">${tags}</p>
    ${aliases}
    <article>${n.fileHtml}</article>
    <section class="backlinks">
      <h3>Backlinks (${n.backlinks.length})</h3>
      <ul>${bl}</ul>
      <h3>Forward (${n.forward.length})</h3>
      <ul>${fw}</ul>
    </section>`;
  // wire roam-link click → in-app nav
  $('#note').querySelectorAll('a.roam-link').forEach((a) => {
    a.addEventListener('click', (ev) => {
      ev.preventDefault();
      selectNode(a.dataset.id);
    });
  });
  $('#note').querySelectorAll('.bl-item').forEach((li) => {
    li.addEventListener('click', () => selectNode(li.dataset.id));
  });
}

async function selectNode(id) {
  state.selectedId = id;
  writeUrlState();
  applyDim();
  if (!id) {
    $('#note').innerHTML = '<p class="placeholder">click a node</p>';
    return;
  }
  try {
    const n = await fetchNode(id);
    renderNotePane(n);
  } catch (e) {
    $('#note').innerHTML = `<pre>${escapeHtml(e.message)}</pre>`;
    return;
  }
  if (state.sigma && state.graph?.hasNode(id)) {
    state.sigma
      .getCamera()
      .animate(state.graph.getNodeAttributes(id), { duration: 400 });
  }
}

// --- Task 18: search ---
function wireSearch() {
  const input = $('#search');
  const ul = $('#search-results');
  let timer = null;
  let cur = -1;
  let items = [];

  const close = () => {
    ul.classList.remove('open');
    cur = -1;
  };
  const open = (results) => {
    items = results;
    ul.innerHTML = results
      .map(
        (n, i) =>
          `<li data-id="${escapeHtml(n.id)}" data-i="${i}">${escapeHtml(n.title)}</li>`,
      )
      .join('');
    ul.classList.toggle('open', results.length > 0);
    cur = -1;
    ul.querySelectorAll('li').forEach((li) => {
      li.addEventListener('click', () => {
        selectNode(li.dataset.id);
        input.value = '';
        close();
      });
    });
  };
  const fire = async (q) => {
    if (!q) {
      close();
      return;
    }
    const res = await fetch(
      `/api/search?q=${encodeURIComponent(q)}&limit=10`,
    );
    if (!res.ok) return;
    open(await res.json());
  };

  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => fire(input.value.trim()), 150);
  });
  input.addEventListener('keydown', (e) => {
    if (!ul.classList.contains('open')) return;
    if (e.key === 'ArrowDown') {
      cur = Math.min(cur + 1, items.length - 1);
    } else if (e.key === 'ArrowUp') {
      cur = Math.max(cur - 1, 0);
    } else if (e.key === 'Enter') {
      if (items[cur]) {
        selectNode(items[cur].id);
        input.value = '';
        close();
      }
      return;
    } else if (e.key === 'Escape') {
      close();
      return;
    } else {
      return;
    }
    e.preventDefault();
    ul.querySelectorAll('li').forEach((li, i) => {
      li.classList.toggle('active', i === cur);
    });
  });
  document.addEventListener('click', (e) => {
    if (!input.contains(e.target) && !ul.contains(e.target)) close();
  });
}

// --- Task 19: tag filter + orphan + dimming ---
function renderTagChips(allTags) {
  const host = $('#tags');
  const chips = allTags
    .map(
      (t) =>
        `<span class="tag-chip" data-tag="${escapeHtml(t)}">${escapeHtml(t)}</span>`,
    )
    .join('');
  host.innerHTML = `${chips}<span class="tag-chip" data-tag="__orphan">✦ orphans</span>`;
  host.querySelectorAll('.tag-chip').forEach((chip) => {
    const t = chip.dataset.tag;
    if (t === '__orphan' && state.orphansOnly) chip.classList.add('active');
    if (state.activeTags.has(t)) chip.classList.add('active');
    chip.addEventListener('click', () => {
      if (t === '__orphan') {
        state.orphansOnly = !state.orphansOnly;
      } else if (state.activeTags.has(t)) {
        state.activeTags.delete(t);
      } else {
        state.activeTags.add(t);
      }
      writeUrlState();
      applyDim();
      renderTagChips(allTags); // refresh active class
    });
  });
}

function applyDim() {
  if (!state.graph) return;
  const wantTags = state.activeTags;
  const wantOrphans = state.orphansOnly;
  const sel = state.selectedId;

  const neighbors = new Set();
  if (sel && state.graph.hasNode(sel)) {
    neighbors.add(sel);
    state.graph.forEachNeighbor(sel, (n) => neighbors.add(n));
  }

  state.graph.forEachNode((id, attr) => {
    let visible = true;
    if (wantTags.size && !attr.tags.some((t) => wantTags.has(t))) visible = false;
    if (wantOrphans && !attr.orphan) visible = false;
    if (sel && !neighbors.has(id)) visible = false;
    state.graph.setNodeAttribute(
      id,
      'color',
      visible ? (attr.orphan ? '#f5a623' : '#5a8dee') : '#1a2030',
    );
  });
  state.graph.forEachEdge((eid, _attr, src, dst) => {
    const visible = !sel || (neighbors.has(src) && neighbors.has(dst));
    state.graph.setEdgeAttribute(eid, 'color', visible ? '#3a3f4d' : '#1a2030');
  });
  state.sigma?.refresh();
}

// --- SSE reload ---
function startSse() {
  const es = new EventSource('/events');
  es.onmessage = (ev) => {
    if (ev.data === 'reload') reloadAll();
  };
  es.onerror = () => {
    /* EventSource auto-reconnects */
  };
}

async function reloadAll() {
  const payload = await fetchGraph();
  state.graph = buildGraphology(payload);
  state.allTags = [...new Set(payload.nodes.flatMap((n) => n.tags))].sort();
  renderTagChips(state.allTags);
  if (state.sigma) {
    state.sigma.kill();
  }
  startLayout(state.graph);
  renderSigma(state.graph);
  $('#stats').textContent = `${payload.nodes.length}n · ${payload.edges.length}e`;
  if (state.selectedId && state.graph.hasNode(state.selectedId)) {
    await selectNode(state.selectedId);
  } else {
    applyDim();
  }
}

async function init() {
  readUrlState();
  await reloadAll();
  wireSearch();
  startSse();
}

init().catch((e) => {
  console.error(e);
  $('#note').innerHTML = `<pre>${escapeHtml(e.message)}</pre>`;
});
