// Structure demo: the real grammar in the browser via web-tree-sitter
// (own dynamic imports, so this section is independent of the lint
// module's load state). Highlighting comes from queries/highlights.scm
// through standard captures; clause bands from a direct tree walk.
import { Parser, Language, Query } from "./pkg/wt/web-tree-sitter.js";

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

// Standard captures -> demo palette (same classes as the query file's
// intent; clause bands come from the tree walk below).
const CAP_CLASS = {
  keyword: "k-Conjunction",
  "punctuation.delimiter": "k-Semicolon",
  number: "k-Number",
  string: "k-Quote",
  "punctuation.special": "k-Ellipsis",
  punctuation: "k-Period",
};

let parser = null;
let query = null;

async function ensure() {
  if (!parser) {
    await Parser.init({ locateFile: (f) => "./pkg/wt/" + f });
    parser = new Parser();
    const lang = await Language.load("./pkg/tree-sitter-english.wasm");
    parser.setLanguage(lang);
    const src = await (await fetch("./queries/highlights.scm")).text();
    query = new Query(lang, src);
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

let ptimer = null;
pinput.addEventListener("input", () => {
  clearTimeout(ptimer);
  ptimer = setTimeout(() => run(false), 250);
});
pinput.addEventListener("scroll", () => {
  pback.scrollTop = pinput.scrollTop;
  pback.scrollLeft = pinput.scrollLeft;
});
prun.addEventListener("click", () => run(true));

let lastP = null;
async function run(force) {
  if (!force && pinput.value === lastP) return;
  try {
    await ensure();
  } catch (e) {
    pback.innerHTML = '<span class="dim">Parser failed to load: ' + e.message + "</span>";
    return;
  }
  lastP = pinput.value;
  const t0 = performance.now();
  const tree = parser.parse(pinput.value);
  const ms = (performance.now() - t0).toFixed(1);
  const errors = tree.rootNode.hasError();
  pstatus.textContent = `parsed in ${ms} ms · live${errors ? " · has ERROR nodes" : ""}`;
  show(pinput.value, tree);
}

function show(full, tree) {
  pback.innerHTML = "";
  treeview.innerHTML = "";
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
  // Token colors from query captures (char space via B); earliest
  // start wins on overlaps.
  const seen = new Set();
  const cols = [];
  for (const c of query.captures(tree.rootNode)) {
    const s = c.node.startIndex;
    if (seen.has(s) || !CAP_CLASS[c.name]) continue;
    seen.add(s);
    cols.push({ s: B(s), e: B(c.node.endIndex), cls: CAP_CLASS[c.name], kind: c.node.type });
  }
  // Clause bands from a direct walk (subordinate wins over enclosing).
  const bands = [];
  const walk = (node) => {
    if (node.type === "subordinate_clause" || node.type === "clause") {
      bands.push({ s: B(node.startIndex), e: B(node.endIndex), sub: node.type === "subordinate_clause" });
    }
    for (let i = 0; i < node.childCount; i++) walk(node.child(i));
  };
  walk(tree.rootNode);

  // Linear sweep: split input at every band/token boundary, then
  // render each segment once (token span nested in band span).
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
      if (c.s <= s && s < c.e && (!col || c.s > col.s)) col = c;
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
      el.className = "tok " + col.cls;
      el.textContent = full.slice(s, e);
      el.title = col.kind;
      parent.appendChild(el);
    } else {
      parent.appendChild(document.createTextNode(full.slice(s, e)));
    }
  }
  frag.appendChild(document.createTextNode("\n"));
  pback.appendChild(frag);

  // Tree view: sentences -> clauses -> tokens from the same parse.
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

run(true).catch((e) => {
  pback.innerHTML = '<span class="dim">Structure demo failed to load: ' + e.message + "</span>";
});
