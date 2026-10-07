import init, { analyze } from "./pkg/english_web.js";

// ---------- Structure demo (pre-parsed native JSON) ----------

async function loadParse() {
  const res = await fetch("parse-examples.json");
  const examples = await res.json();
  const picker = document.getElementById("examples");
  const ptree = document.getElementById("ptree");
  const treeview = document.getElementById("treeview");
  examples.forEach((ex, i) => {
    const b = document.createElement("button");
    b.textContent = ex.label;
    if (i === 0) b.classList.add("on");
    b.addEventListener("click", () => {
      picker.querySelectorAll("button").forEach((x) => x.classList.remove("on"));
      b.classList.add("on");
      show(ex);
    });
    picker.appendChild(b);
  });
  function show(ex) {
    ptree.innerHTML = "";
    treeview.innerHTML = "";
    for (const s of ex.sentences) {
      for (const c of s.clauses) {
        const cspan = document.createElement("span");
        cspan.className = "clause" + (c.sub ? " sub" : "");
        cspan.title = c.sub ? "subordinate clause" : "coordinate clause";
        // Render tokens in order, preserving inter-token text via spans.
        let cur = c.s;
        for (const t of c.tokens) {
          if (t.s > cur) cspan.appendChild(document.createTextNode(ex.text.slice(cur, t.s)));
          const el = document.createElement("span");
          el.className = "tok k-" + t.k;
          el.textContent = ex.text.slice(t.s, t.e);
          el.title = t.k;
          cspan.appendChild(el);
          cur = t.e;
        }
        if (cur < c.e) cspan.appendChild(document.createTextNode(ex.text.slice(cur, c.e)));
        ptree.appendChild(cspan);
      }
    }
    for (const s of ex.sentences) {
      const d = document.createElement("details");
      const sum = document.createElement("summary");
      sum.textContent = `sentence · ${s.clauses.length} clause(s)`;
      d.appendChild(sum);
      for (const c of s.clauses) {
        const cd = document.createElement("details");
        const cs = document.createElement("summary");
        cs.textContent = c.sub ? "subordinate clause" : "clause";
        cd.appendChild(cs);
        const ul = document.createElement("ul");
        for (const t of c.tokens) {
          const li = document.createElement("li");
          const code = document.createElement("code");
          code.textContent = t.k;
          li.appendChild(code);
          li.appendChild(document.createTextNode(" " + ex.text.slice(t.s, t.e)));
          ul.appendChild(li);
        }
        cd.appendChild(ul);
        d.appendChild(cd);
      }
      treeview.appendChild(d);
    }
  }
  show(examples[0]);
}

loadParse().catch((e) => {
  document.getElementById("ptree").textContent = "failed to load examples: " + e;
});

// ---------- Lint demo (live WebAssembly) ----------

let ready = false;
const status = document.getElementById("status");
const input = document.getElementById("input");
const findingsEl = document.getElementById("findings");
const sentencesEl = document.getElementById("sentences");

init().then(() => {
  ready = true;
  status.textContent = "ready (local WASM)";
  run();
}).catch((e) => {
  status.textContent = "failed to load WASM: " + e;
});

document.getElementById("run").addEventListener("click", run);

function run() {
  if (!ready) {
    status.textContent = "still loading…";
    return;
  }
  const t0 = performance.now();
  const out = JSON.parse(analyze(input.value));
  const ms = (performance.now() - t0).toFixed(1);
  status.textContent = `analyzed in ${ms} ms`;

  findingsEl.innerHTML = "";
  if (out.findings.length === 0) {
    findingsEl.innerHTML = '<li class="dim">No findings.</li>';
  }
  for (const f of out.findings) {
    const li = document.createElement("li");
    li.textContent = `[${f.rule}] ${f.line}:${f.col} — ${f.message}`;
    findingsEl.appendChild(li);
  }

  sentencesEl.innerHTML = "";
  for (const s of out.sentences) {
    const div = document.createElement("div");
    div.className = "sent";
    for (const p of s.pieces) {
      const span = document.createElement("span");
      span.className = "ptok chunk-" + p.chunk;
      span.title = `${p.tag} · ${p.chunk}`;
      span.textContent = p.w;
      div.appendChild(span);
      div.appendChild(document.createTextNode(" "));
    }
    sentencesEl.appendChild(div);
  }
}
