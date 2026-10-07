// Structure demo: the real grammar in the browser via web-tree-sitter
// (own dynamic imports, so this section is independent of the lint
// module's load state). Words take POS colors from the tagger
// (wasm-bindgen build of the exact native pipeline); structural roles
// that POS can't see (joins, quotes, ellipsis) come from
// queries/highlights.scm; clause bands from a direct tree walk.
import { Parser, Language, Query } from "./pkg/wt/web-tree-sitter.js";
import githubLight from "./pkg/th/theme-github-light.js";
import githubDark from "./pkg/th/theme-github-dark.js";

const pinput = document.getElementById("pinput");
const prun = document.getElementById("prun");
const pstatus = document.getElementById("pstatus");
const picker = document.getElementById("examples");
const pback = document.getElementById("pback");
const treeview = document.getElementById("treeview");

const SHOWCASE = [
  ["Simple sentence", "Time flies like an arrow."],
  ["Abbreviations and time", "Mr. Smith arrived at 10:30, and Mrs. Jones left because the meeting ended."],
  ["Em-dash join", "She laughed — and then she cried."],
  ["Subordinator and semicolon", "The book that I read was good; it cost $5."],
  ["Parenthetical", "The whale (a huge beast) swam on."],
  ["Colon elaboration", "He had one goal: to win."],
];

// UPOS tag -> palette class (grouped; see legend).
const TAG_CLASS = {
  NOUN: "t-noun", PROPN: "t-noun", VERB: "t-verb", AUX: "t-verb",
  ADJ: "t-adj", ADV: "t-adv", ADP: "t-adp", DET: "t-det", PRON: "t-pron",
  CCONJ: "t-conj", SCONJ: "t-sconj", NUM: "t-num", PART: "t-part",
  INTJ: "t-intj", PUNCT: "t-punct", SYM: "t-x", X: "t-x",
};

// Query captures that ADD information POS lacks (join roles, quote and
// ellipsis classes). Conjunction/subordinator keywords already read
// distinctly from CCONJ/SCONJ tags, so keyword captures stay unused.
const STRUCT_CAPTURES = new Set([
  "punctuation.delimiter",
  "string",
  "punctuation.special",
  "punctuation",
]);

// Active treelight theme; resolved per capture with the standard
// dotted-name fallback (punctuation.delimiter -> punctuation).
let theme = githubLight;
function resolveStyle(name) {
  let t = name;
  while (t) {
    if (theme.styles[t]) return theme.styles[t];
    const i = t.lastIndexOf(".");
    if (i < 0) break;
    t = t.slice(0, i);
  }
  return null;
}

let parser = null;
let query = null;
let analyzeFn = null;

async function ensure() {
  if (!parser) {
    await Parser.init({ locateFile: (f) => "./pkg/wt/" + f });
    parser = new Parser();
    const lang = await Language.load("./pkg/tree-sitter-english.wasm");
    parser.setLanguage(lang);
    const src = await (await fetch("./queries/highlights.scm")).text();
    query = new Query(lang, src);
  }
  if (!analyzeFn) {
    const mod = await import("./pkg/english_web.js");
    await mod.default();
    analyzeFn = mod.analyze;
  }
}

SHOWCASE.forEach(([label, text], i) => {
  const b = document.createElement("button");
  b.textContent = label;
  if (i === 0) b.classList.add("on");
  b.addEventListener("click", () => {
    picker.querySelectorAll("button").forEach((x) => x.classList.remove("on"));
    b.classList.add("on");
    pinput.value = text;
    run(true);
  });
  picker.appendChild(b);
});

// Two speeds: highlight follows typing fast; the tree view (expensive
// DOM) refreshes only on pause or explicit Parse.
let fastTimer = null;
let slowTimer = null;
pinput.addEventListener("input", () => {
  clearTimeout(fastTimer);
  clearTimeout(slowTimer);
  fastTimer = setTimeout(() => run(false, true), 100);
  slowTimer = setTimeout(() => run(false, false), 800);
});
pinput.addEventListener("scroll", () => {
  pback.scrollTop = pinput.scrollTop;
  pback.scrollLeft = pinput.scrollLeft;
});
prun.addEventListener("click", () => run(true, false));

for (const [id, th, dark] of [["theme-light", githubLight, false], ["theme-dark", githubDark, true]]) {
  document.getElementById(id).addEventListener("click", (ev) => {
    document.querySelectorAll(".theme-picker button").forEach((x) => x.classList.remove("on"));
    ev.target.classList.add("on");
    theme = th;
    // The switch must be unmistakable: backdrop follows the theme
    // (dark editors are dark), token colors resolve from it.
    document.querySelector("#parse .editor pre").style.background = dark ? "#0d1117" : "";
    document.querySelector("#parse .editor pre").style.color = dark ? "#e6edf3" : "";
    run(true, true);
  });
}

let lastP = null;
let lastOut = null;
async function run(force, highlightOnly) {
  if (!force && pinput.value === lastP && (highlightOnly || lastOut?.treed)) return;
  try {
    await ensure();
  } catch (e) {
    pback.innerHTML = '<span class="dim">Parser failed to load: ' + e.message + "</span>";
    return;
  }
  lastP = pinput.value;
  const t0 = performance.now();
  let tree, pos;
  try {
    tree = parser.parse(pinput.value);
    pos = JSON.parse(analyzeFn(pinput.value));
  } catch (e) {
    pstatus.textContent = "analysis error: " + e.message;
    return;
  }
  const ms = (performance.now() - t0).toFixed(1);
  showHighlight(pinput.value, tree, pos);
  // ERROR honesty: a trailing fragment without an end mark is
  // incomplete input (normal mid-keystroke state), not a grammar
  // failure. Flag ERROR only when it appears in completed text.
  const errs = [];
  const findErr = (node, sentIdx) => {
    if (node.type === "ERROR") errs.push(sentIdx);
    for (let i = 0; i < node.childCount; i++) findErr(node.child(i), sentIdx);
  };
  const topSents = [];
  const collectS = (node) => {
    if (node.type === "sentence") topSents.push(node);
    else for (let i = 0; i < node.childCount; i++) collectS(node.child(i));
  };
  collectS(tree.rootNode);
  topSents.forEach((s, i) => findErr(s, i));
  const terminated = /[.?!…]\s*$/.test(pinput.value);
  const realErr = errs.some((i) => terminated || i < topSents.length - 1);
  pstatus.textContent = `parsed in ${ms} ms · live${realErr ? " · has ERROR nodes" : ""}`;
  if (!highlightOnly) {
    showTree(pinput.value, tree);
    lastOut = { treed: true };
  } else {
    lastOut = { treed: false };
  }
}

function showHighlight(full, tree, pos) {
  pback.innerHTML = "";
  // tree-sitter spans are BYTE offsets; JS slices UTF-16 units. Map
  // once so multibyte punctuation (em-dashes, quotes) can't drift.
  const enc = new TextEncoder();
  const nbytes = enc.encode(full).length;
  const b2c = new Array(nbytes + 1);
  {
    let ci = 0, bp = 0;
    for (const ch of full) {
      const bl = enc.encode(ch).length;
      for (let k = 0; k < bl; k++) b2c[bp + k] = ci;
      bp += bl; ci++;
    }
    b2c[nbytes] = ci;
  }
  const B = (x) => b2c[Math.min(Math.max(x, 0), nbytes)];
  // POS colors (char space).
  const cols = [];
  for (const s of pos.sentences) {
    for (const p of s.pieces) {
      const cls = TAG_CLASS[p.tag];
      if (cls) cols.push({ s: B(p.s), e: B(p.e), cls, kind: `${p.tag} · ${p.chunk}` });
    }
  }
  // Structural overrides (char space); earliest start wins. Colors
  // come from the active treelight theme, not the local palette.
  const seen = new Set();
  for (const c of query.captures(tree.rootNode)) {
    const key = c.node.startIndex;
    if (seen.has(key) || !STRUCT_CAPTURES.has(c.name)) continue;
    seen.add(key);
    const st = resolveStyle(c.name);
    if (!st || !st.fg) continue;
    cols.push({
      s: B(c.node.startIndex),
      e: B(c.node.endIndex),
      fg: st.fg,
      bold: !!st.bold,
      italic: !!st.italic,
      kind: c.node.type,
      struct: true,
    });
  }
  // Clause bands from a direct walk.
  const bands = [];
  const walk = (node) => {
    if (node.type === "subordinate_clause" || node.type === "clause") {
      bands.push({ s: B(node.startIndex), e: B(node.endIndex), sub: node.type === "subordinate_clause" });
    }
    for (let i = 0; i < node.childCount; i++) walk(node.child(i));
  };
  walk(tree.rootNode);

  // Linear sweep: split input at every boundary, render each segment
  // once (token span nested in band span). Structural spans win over
  // POS spans on overlap.
  const cuts = new Set([0, full.length]);
  for (const b of bands) { cuts.add(b.s); cuts.add(b.e); }
  for (const c of cols) { cuts.add(c.s); cuts.add(c.e); }
  const pts = [...cuts].filter((x) => x >= 0 && x <= full.length).sort((a, b) => a - b);
  const frag = document.createDocumentFragment();
  let curBand = null;
  let curBandEl = null;
  for (let i = 0; i < pts.length - 1; i++) {
    const s = pts[i], e = pts[i + 1];
    if (s >= e) continue;
    let band = null;
    for (const b of bands) {
      if (b.s <= s && e <= b.e && (!band || b.s >= band.s)) band = b;
    }
    let col = null;
    for (const c of cols) {
      if (c.s <= s && s < c.e && (!col || c.s > col.s || (c.struct && !col.struct))) col = c;
    }
    if (band !== curBand) {
      curBand = band;
      curBandEl = null;
      if (band) {
        curBandEl = document.createElement("span");
        curBandEl.className = "clause" + (band.sub ? " sub" : "");
        curBandEl.title = band.sub ? "subordinate clause" : "coordinate clause";
        frag.appendChild(curBandEl);
      }
    }
    const parent = curBandEl || frag;
    if (col) {
      const el = document.createElement("span");
      if (col.fg) {
        el.className = "tok st";
        el.style.color = col.fg;
        if (col.bold) el.style.fontWeight = "700";
        if (col.italic) el.style.fontStyle = "italic";
      } else {
        el.className = "tok " + col.cls;
      }
      el.textContent = full.slice(s, e);
      el.title = col.kind;
      parent.appendChild(el);
    } else {
      parent.appendChild(document.createTextNode(full.slice(s, e)));
    }
  }
  frag.appendChild(document.createTextNode("\n"));
  pback.appendChild(frag);
}

function showTree(full, tree) {
  treeview.innerHTML = "";
  const sents = [];
  const collect = (node) => {
    if (node.type === "sentence") sents.push(node);
    for (let i = 0; i < node.childCount; i++) collect(node.child(i));
  };
  collect(tree.rootNode);
  for (const s of sents) {
    const d = document.createElement("details");
    const sum = document.createElement("summary");
    sum.textContent = "sentence";
    d.appendChild(sum);
    const walkClauses = (node) => {
      if (node.type === "clause" || node.type === "subordinate_clause") {
        const cd = document.createElement("details");
        const cs = document.createElement("summary");
        cs.textContent = node.type === "subordinate_clause" ? "subordinate clause" : "clause";
        cd.appendChild(cs);
        const ul = document.createElement("ul");
        const toks = [];
        const gather = (n) => {
          if (n.childCount === 0) toks.push(n);
          else for (let i = 0; i < n.childCount; i++) gather(n.child(i));
        };
        gather(node);
        for (const t of toks) {
          if (!t.text.trim()) continue;
          const li = document.createElement("li");
          const code = document.createElement("code");
          code.textContent = t.type;
          li.appendChild(code);
          li.appendChild(document.createTextNode(" " + t.text));
          ul.appendChild(li);
        }
        cd.appendChild(ul);
        d.appendChild(cd);
      } else {
        for (let i = 0; i < node.childCount; i++) walkClauses(node.child(i));
      }
    };
    walkClauses(s);
    treeview.appendChild(d);
  }
}

run(true, false).catch((e) => {
  pback.innerHTML = '<span class="dim">Structure demo failed to load: ' + e.message + "</span>";
});
