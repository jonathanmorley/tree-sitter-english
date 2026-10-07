// Structure demo: pre-parsed native JSON only — no WebAssembly import,
// so this section renders even if the WASM bundle fails to load.
async function loadParse() {
  const res = await fetch("parse-examples.json");
  if (!res.ok) throw new Error(`examples HTTP ${res.status}`);
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
  document.getElementById("ptree").innerHTML =
    '<span class="dim">Structure examples failed to load: ' + e.message + "</span>";
});
