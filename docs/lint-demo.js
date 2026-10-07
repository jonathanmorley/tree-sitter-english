import init, { analyze } from "./pkg/english_web.js";

// Lint demo: live WebAssembly. Analyzes on every keystroke (debounced)
// plus the button, for keyboards that don't fire `input` on paste.
let ready = false;
let lastText = null;
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

document.getElementById("run").addEventListener("click", () => {
  lastText = null;
  run();
});

let timer = null;
input.addEventListener("input", () => {
  clearTimeout(timer);
  timer = setTimeout(run, 250);
});

function run() {
  if (!ready) {
    status.textContent = "still loading…";
    return;
  }
  if (input.value === lastText) return;
  lastText = input.value;
  const t0 = performance.now();
  let out;
  try {
    out = JSON.parse(analyze(input.value));
  } catch (e) {
    status.textContent = "analysis error: " + e.message;
    return;
  }
  const ms = (performance.now() - t0).toFixed(1);
  status.textContent = `analyzed in ${ms} ms · live`;

  findingsEl.innerHTML = "";
  if (out.findings.length === 0) {
    findingsEl.innerHTML = '<li class="dim">No findings.</li>';
  }
  for (const f of out.findings) {
    const li = document.createElement("li");
    const badge = document.createElement("span");
    badge.className = "rule rule-" + f.rule.replace(".", "-");
    badge.textContent = f.rule;
    li.appendChild(badge);
    li.appendChild(document.createTextNode(` ${f.line}:${f.col} — ${f.message}`));
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
