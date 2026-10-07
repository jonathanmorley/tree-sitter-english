// Structure demo: live native parse in WebAssembly (own dynamic import,
// so this section is independent of the lint module's load state).
let analyzeFn = null;
const pinput = document.getElementById("pinput");
const prun = document.getElementById("prun");
const pstatus = document.getElementById("pstatus");
const picker = document.getElementById("examples");
const ptree = document.getElementById("ptree");
const treeview = document.getElementById("treeview");

const SHOWCASE = [
  ["Simple sentence", "Time flies like an arrow."],
  ["Abbreviations and time", "Mr. Smith arrived at 10:30, and Mrs. Jones left because the meeting ended."],
  ["Em-dash join", "She laughed — and then she cried."],
  ["Subordinator and semicolon", "The book that I read was good; it cost $5."],
  ["Parenthetical", "The whale (a huge beast) swam on."],
  ["Colon elaboration", "He had one goal: to win."],
];

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

async function ensure() {
  if (!analyzeFn) {
    const mod = await import("./pkg/english_web.js");
    await mod.default();
    analyzeFn = mod.analyze;
  }
}

let ptimer = null;
pinput.addEventListener("input", () => {
  clearTimeout(ptimer);
  ptimer = setTimeout(() => run(false), 250);
});
prun.addEventListener("click", () => run(true));

let lastP = null;
async function run(force) {
  if (!force && pinput.value === lastP) return;
  try {
    await ensure();
  } catch (e) {
    ptree.innerHTML = '<span class="dim">Parser failed to load: ' + e.message + "</span>";
    return;
  }
  lastP = pinput.value;
  const t0 = performance.now();
  const out = JSON.parse(analyzeFn(pinput.value));
  pstatus.textContent = `parsed in ${(performance.now() - t0).toFixed(1)} ms · live`;
  show(pinput.value, out);
}

function show(full, out) {
  ptree.innerHTML = "";
  treeview.innerHTML = "";
  if (out.sentences.length === 0) {
    ptree.innerHTML = '<span class="dim">No sentences — type something.</span>';
    return;
  }
  for (const s of out.sentences) {
    for (const c of s.tree) {
      const cspan = document.createElement("span");
      cspan.className = "clause" + (c.sub ? " sub" : "");
      cspan.title = c.sub ? "subordinate clause" : "coordinate clause";
      let cur = c.s;
      for (const t of c.tokens) {
        if (t.s > cur) cspan.appendChild(document.createTextNode(full.slice(cur, t.s)));
        const el = document.createElement("span");
        el.className = "tok k-" + t.k;
        el.textContent = full.slice(t.s, t.e);
        el.title = t.k;
        cspan.appendChild(el);
        cur = t.e;
      }
      if (cur < c.e) cspan.appendChild(document.createTextNode(full.slice(cur, c.e)));
      ptree.appendChild(cspan);
    }
  }
  for (const s of out.sentences) {
    const d = document.createElement("details");
    const sum = document.createElement("summary");
    sum.textContent = `sentence · ${s.clauses} clause(s), ${s.subords} subordinate`;
    d.appendChild(sum);
    for (const c of s.tree) {
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
        li.appendChild(document.createTextNode(" " + full.slice(t.s, t.e)));
        ul.appendChild(li);
      }
      cd.appendChild(ul);
      d.appendChild(cd);
    }
    treeview.appendChild(d);
  }
}

run(true).catch((e) => {
  ptree.innerHTML = '<span class="dim">Structure demo failed to load: ' + e.message + "</span>";
});
