// Harmonize UD trains toward CANONICAL.md decisions (M1-M6 + J3).
// Rule rationale and measurements live there; this script only
// implements the accepted set. Reads CoNLL-U, rewrites UPOS col 4
// where rules fire, reports counts.
// Usage: node harmonize.js <in.conllu> <out.conllu> <lines|gum>
const fs = require('fs');
const [inF, outF, tb] = process.argv.slice(2);

const M1_FORMS = new Set(['Mr', 'Mr.', 'Mrs', 'Mrs.', 'President', 'Jews', 'Lord', 'Sir']);
const M6_NEXT = new Set(['Kingdom', 'States', 'Airlines', 'Nations']);

const counts = {};
const bump = r => { counts[r] = (counts[r] || 0) + 1; };

function tok(line) {
  const c = line.split('\t');
  return { cols: c, form: c[1], upos: c[3], deprel: c[7] };
}
function isTok(line) {
  if (line === '' || line.startsWith('#')) return false;
  const c = line.split('\t');
  return c.length >= 5 && !c[0].includes('-') && !c[0].includes('.');
}

const lines = fs.readFileSync(inF, 'utf8').split('\n');
const out = [];
let sent = [];
const flush = () => {
  for (let i = 0; i < sent.length; i++) {
    const t = tok(sent[i]);
    const prev = sent.slice(0, i).filter(isTok).map(tok).pop();
    const next = sent.slice(i + 1).filter(isTok).map(tok)[0];
    let tag = t.upos, rule = null;
    if (tb === 'lines') {
      if (t.upos === 'NOUN' && t.deprel === 'nmod:desc' && M1_FORMS.has(t.form)) { tag = 'PROPN'; rule = 'M1'; }
      else if (t.form === 'ago' && t.upos === 'ADP') { tag = 'ADV'; rule = 'M2'; }
      else if (t.form === 'else' && t.upos === 'ADJ') { tag = 'ADV'; rule = 'M3'; }
      else if (t.form === 'another') { tag = 'DET'; rule = 'M4'; }
      else if (t.form === 'course' && t.upos === 'ADV' && prev && prev.form === 'of') { tag = 'NOUN'; rule = 'M5'; }
      // J3 (ADV canonical): LinES tags temporal-subordinate `when`
      // SCONJ; EWT lumps all `when` as ADV (304:0). Convert wholesale.
      else if (t.form === 'when' && t.upos === 'SCONJ') { tag = 'ADV'; rule = 'J3'; }
    }
    if (tb === 'gum') {
      if (t.form === 'United' && t.upos === 'VERB' && t.deprel === 'amod' && next && M6_NEXT.has(next.form)) { tag = 'ADJ'; rule = 'M6'; }
    }
    if (rule) { bump(`${rule}:${t.form}:${t.upos}->${tag}`); t.cols[3] = tag; out.push(t.cols.join('\t')); }
    else out.push(sent[i]);
  }
  sent = [];
};
for (const line of lines) {
  if (line === '' && sent.length === 0) { out.push(line); continue; }
  if (line === '') { flush(); out.push(line); continue; }
  if (!isTok(line)) {
    // comments / MWT / empty nodes: pass through (flush pending? no -
    // sentence continues; just buffer)
    sent.push(line); continue;
  }
  sent.push(line);
}
flush();
fs.writeFileSync(outF, out.join('\n'));
console.log(`wrote ${outF}`);
for (const [k, n] of Object.entries(counts).sort((a, b) => b[1] - a[1])) console.log(`  ${n}x ${k}`);
