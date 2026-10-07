import init, { analyze } from "./pkg/english_web.js";

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

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

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
      span.className = "tok chunk-" + p.chunk;
      span.title = `${p.tag} · ${p.chunk}`;
      span.textContent = p.w;
      div.appendChild(span);
      div.appendChild(document.createTextNode(" "));
    }
    sentencesEl.appendChild(div);
  }
}
