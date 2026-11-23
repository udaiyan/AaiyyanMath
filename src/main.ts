import { invoke } from '@tauri-apps/api/core';
import 'mathlive';
import type { MathfieldElement } from 'mathlive';

type R = { id: number; latex: string; python: string; x: number; y: number;
           value: string | null; error: string | null; assigns: boolean };
type S = { regions: R[]; python: string; errors: number; ms: number };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const world = $('world'), py = $<HTMLTextAreaElement>('py');
let s: S | null = null, sel: number | null = null;
const fields = new Map<number, MathfieldElement>();
let typingPy = false, pyTimer = 0;

const OPTS = { smartMode: false, smartFence: false, smartSuperscript: false,
  virtualKeyboardMode: 'manual' as const, macros: { coloneqq: ':=' } };

const GROUPS: [string, [string, string, string][]][] = [
  ['Structure', [
    ['x:=', ':=', 'Define'], ['a/b', '\\frac{#@}{#?}', 'Fraction'],
    ['xⁿ', '^{#?}', 'Power'], ['√', '\\sqrt{#?}', 'Root'],
    ['ⁿ√', '\\sqrt[#?]{#@}', 'n-th root'], ['( )', '\\left(#@\\right)', 'Group'],
    ['|x|', '\\left|#@\\right|', 'Absolute value'],
  ]],
  ['Calculus', [
    ['∫', '\\int_{#?}^{#?}#?\\,dx', 'Definite integral — Tab moves between slots'],
    ['∑', '\\sum_{k=#?}^{#?}#?', 'Sum'],
    ['∏', '\\prod_{k=#?}^{#?}#?', 'Product'],
  ]],
  ['Functions', [
    ['sin', '\\sin\\left(#@\\right)', 'sine'], ['cos', '\\cos\\left(#@\\right)', 'cosine'],
    ['tan', '\\tan\\left(#@\\right)', 'tangent'], ['ln', '\\ln\\left(#@\\right)', 'natural log'],
    ['exp', '\\exp\\left(#@\\right)', 'exponential'], ['abs', '\\left|#@\\right|', 'magnitude'],
    ['mean', '\\mathrm{mean}\\left(#@\\right)', 'mean'], ['std', '\\mathrm{std}\\left(#@\\right)', 'std dev'],
  ]],
  ['Symbols', [
    ['π', '\\pi', 'pi'], ['e', 'e', 'Euler number'], ['∞', '\\infty', 'infinity'],
    ['α', '\\alpha', 'alpha'], ['β', '\\beta', 'beta'], ['θ', '\\theta', 'theta'],
    ['λ', '\\lambda', 'lambda'], ['σ', '\\sigma', 'sigma'], ['ω', '\\omega', 'omega'],
  ]],
  ['Operators', [['+', '+', 'add'], ['−', '-', 'subtract'], ['·', '\\cdot ', 'multiply'], ['÷', '/', 'divide']]],
];

function tools() {
  const t = $('tools'); t.innerHTML = '';
  for (const [name, keys] of GROUPS) {
    const h = document.createElement('h4'); h.textContent = name; t.appendChild(h);
    const box = document.createElement('div'); box.className = 'keys';
    for (const [label, latex, tip] of keys) {
      const b = document.createElement('button');
      b.textContent = label; b.title = tip;
      b.onmousedown = (e) => e.preventDefault();
      b.onclick = () => place(latex);
      box.appendChild(b);
    }
    t.appendChild(box);
  }
  const h = document.createElement('h4'); h.textContent = 'Display'; t.appendChild(h);

  const control = (label: string, min: number, max: number, value: number,
                   unit: string, apply: (v: number) => void) => {
    const wrap = document.createElement('label'); wrap.className = 'ctl';
    const lab = document.createElement('span'); lab.className = 'lab';
    const name = document.createElement('span'); name.textContent = label;
    const out = document.createElement('b'); out.textContent = value + unit;
    lab.append(name, out);
    const r = document.createElement('input');
    r.type = 'range'; r.min = String(min); r.max = String(max); r.value = String(value);
    r.oninput = () => { const v = Number(r.value); out.textContent = v + unit; apply(v); };
    wrap.append(lab, r); t.appendChild(wrap);
    apply(value);
  };

  control('Maths size', 16, 48, 26, 'px',
    (v) => document.documentElement.style.setProperty('--math', v + 'px'));
}

function focused(): MathfieldElement | null {
  if (sel !== null) { const f = fields.get(sel); if (f) return f; }
  return null;
}

/** Put the caret inside the first empty slot so it is obvious where to type. */
function firstSlot(f: MathfieldElement) {
  requestAnimationFrame(() => {
    f.focus();
    try { f.executeCommand('moveToNextPlaceholder'); } catch { /* no slots */ }
  });
}

async function place(latex: string) {
  const f = focused();
  if (f) {
    f.focus();
    f.insert(latex, {
      insertionMode: 'replaceSelection',
      selectionMode: 'placeholder',
      focus: true,
    });
    await push(f);
    firstSlot(f);
    return;
  }
  s = await invoke<S>('add_region', {
    latex, x: 60, y: 60 + (s?.regions.length ?? 0) * 76,
  });
  const id = s.regions[s.regions.length - 1].id;
  sel = id;
  render();
  const nf = fields.get(id);
  if (nf) firstSlot(nf);
}

async function push(f: MathfieldElement) {
  s = await invoke<S>('set_latex', { id: Number(f.dataset.id), latex: f.value });
  render();
}

function render() {
  if (!s) return;
  const live = new Set(s.regions.map((r) => r.id));
  for (const el of [...world.children] as HTMLElement[]) {
    const id = Number(el.dataset.id);
    if (!live.has(id)) { el.remove(); fields.delete(id); }
  }
  for (const r of s.regions) {
    let el = world.querySelector<HTMLElement>(`[data-id="${r.id}"]`);
    if (!el) { el = make(r); world.appendChild(el); }
    el.style.left = r.x + 'px'; el.style.top = r.y + 'px';
    el.classList.toggle('sel', sel === r.id);
    const f = fields.get(r.id)!;
    if (document.activeElement !== f && f.value !== r.latex) f.value = r.latex;
    const v = el.querySelector<HTMLElement>('.val')!;
    v.textContent = r.value !== null ? '= ' + r.value : '';
    const e = el.querySelector<HTMLElement>('.err')!;
    e.textContent = r.error ?? '';
    e.style.display = r.error ? '' : 'none';
  }
  $('hint').style.display = s.regions.length ? 'none' : '';
  if (!typingPy && document.activeElement !== py) py.value = s.python;
  $('stat').textContent = `${s.regions.length} regions · ${s.errors} errors · ${s.ms.toFixed(1)} ms`;
}

function make(r: R): HTMLElement {
  const el = document.createElement('div');
  el.className = 'region'; el.dataset.id = String(r.id);

  const grip = document.createElement('div'); grip.className = 'grip'; grip.title = 'Drag to move';
  let drag: { dx: number; dy: number } | null = null;
  grip.onpointerdown = (e) => {
    e.stopPropagation(); grip.setPointerCapture(e.pointerId);
    drag = { dx: e.clientX - r.x, dy: e.clientY - r.y };
  };
  grip.onpointermove = (e) => {
    if (!drag) return;
    el.style.left = Math.round((e.clientX - drag.dx) / 12) * 12 + 'px';
    el.style.top = Math.round((e.clientY - drag.dy) / 12) * 12 + 'px';
  };
  grip.onpointerup = async (e) => {
    if (!drag) return; drag = null; grip.releasePointerCapture(e.pointerId);
    s = await invoke<S>('move_region', { id: r.id, x: parseFloat(el.style.left), y: parseFloat(el.style.top) });
    render();
  };
  el.appendChild(grip);

  const f = document.createElement('math-field') as MathfieldElement;
  f.dataset.id = String(r.id); f.setOptions(OPTS); f.value = r.latex;
  f.addEventListener('input', () => { void push(f); });
  f.addEventListener('focus', () => { sel = r.id; mark(); });
  f.addEventListener('keydown', (ev) => {
    if (ev.key === 'Tab') {
      ev.preventDefault();
      try {
        f.executeCommand(ev.shiftKey ? 'moveToPreviousPlaceholder' : 'moveToNextPlaceholder');
      } catch { /* no slots left */ }
      return;
    }
    if (ev.key === 'Escape') { f.blur(); sel = null; mark(); }
  });
  fields.set(r.id, f); el.appendChild(f);

  const val = document.createElement('span'); val.className = 'val'; el.appendChild(val);
  const err = document.createElement('span'); err.className = 'err'; el.appendChild(err);

  const del = document.createElement('button');
  del.className = 'del';
  del.title = 'Delete region';
  del.setAttribute('aria-label', 'Delete region');
  del.innerHTML = '<svg viewBox="0 0 24 24" width="15" height="15" fill="none" '
    + 'stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
    + '<path d="M3.5 6h17"/>'
    + '<path d="M9.5 6V4.3A1.3 1.3 0 0 1 10.8 3h2.4a1.3 1.3 0 0 1 1.3 1.3V6"/>'
    + '<path d="M5.8 6l.9 13.2A1.8 1.8 0 0 0 8.5 21h7a1.8 1.8 0 0 0 1.8-1.8L18.2 6"/>'
    + '<path d="M10.2 10.5v6.2M13.8 10.5v6.2"/>'
    + '</svg>';
  del.onclick = async (ev) => { ev.stopPropagation(); s = await invoke<S>('remove_region', { id: r.id }); sel = null; render(); };
  el.appendChild(del);

  el.onpointerdown = (e) => { if (!(e.target as HTMLElement).closest('.grip,.del')) { sel = r.id; mark(); } };
  return el;
}

function mark() {
  for (const el of world.children as HTMLCollectionOf<HTMLElement>)
    el.classList.toggle('sel', Number(el.dataset.id) === sel);
}

// canvas click -> new region at that point
$('canvas').addEventListener('pointerdown', async (e) => {
  if ((e.target as HTMLElement).closest('.region')) return;
  const box = world.getBoundingClientRect();
  const x = Math.round((e.clientX - box.left) / 12) * 12;
  const y = Math.round((e.clientY - box.top) / 12) * 12;
  s = await invoke<S>('add_region', { latex: '', x, y });
  const id = s.regions[s.regions.length - 1].id;
  sel = id; render();
  requestAnimationFrame(() => fields.get(id)?.focus());
});

// python -> canvas, debounced
py.addEventListener('focus', () => { typingPy = true; });
py.addEventListener('blur', () => { typingPy = false; render(); });
py.addEventListener('input', () => {
  window.clearTimeout(pyTimer);
  pyTimer = window.setTimeout(async () => { s = await invoke<S>('set_python', { text: py.value }); render(); }, 280);
});

// splitters
function splitter(el: HTMLElement, vertical: boolean, cssVar: string, min: number, max: number, invert = false) {
  el.addEventListener('pointerdown', (e) => {
    e.preventDefault(); el.setPointerCapture(e.pointerId); el.classList.add('act');
    const start = vertical ? e.clientX : e.clientY;
    const cur = parseFloat(getComputedStyle(document.documentElement).getPropertyValue(cssVar));
    const move = (ev: PointerEvent) => {
      const d = (vertical ? ev.clientX : ev.clientY) - start;
      const v = Math.min(max, Math.max(min, cur + (invert ? -d : d)));
      document.documentElement.style.setProperty(cssVar, v + 'px');
      localStorage.setItem(cssVar, String(v));
    };
    const up = (ev: PointerEvent) => {
      el.classList.remove('act'); el.releasePointerCapture(ev.pointerId);
      el.removeEventListener('pointermove', move); el.removeEventListener('pointerup', up);
    };
    el.addEventListener('pointermove', move); el.addEventListener('pointerup', up);
  });
}
splitter($('splitL'), true, '--tools', 140, 460);
splitter($('splitB'), false, '--code', 100, 620, true);

// pane visibility
const panes: [string, string, string, () => void][] = [
  ['tTools', 'grid', 'no-tools', () => {}],
  ['tCanvas', 'main', 'no-canvas', () => {}],
  ['tCode', 'main', 'no-code', () => {}],
];
for (const [btn, host, cls] of panes) {
  const b = $(btn), h = $(host);
  const saved = localStorage.getItem(cls);
  if (saved === '1') { h.classList.add(cls); b.classList.remove('on'); }
  b.onclick = () => {
    const on = h.classList.toggle(cls);
    b.classList.toggle('on', !on);
    localStorage.setItem(cls, on ? '1' : '0');
  };
}
for (const v of ['--tools', '--code']) {
  const saved = localStorage.getItem(v);
  if (saved) document.documentElement.style.setProperty(v, saved + 'px');
}

$('add').onclick = () => place('');
$('undo').onclick = async () => { s = await invoke<S>('undo'); render(); };
$('redo').onclick = async () => { s = await invoke<S>('redo'); render(); };
$('copy').onclick = async () => { await navigator.clipboard.writeText(await invoke<string>('script')); };

window.addEventListener('keydown', (e) => {
  const cmd = e.ctrlKey || e.metaKey;
  if (cmd && e.key === '1') { e.preventDefault(); $('tTools').click(); }
  if (cmd && e.key === '2') { e.preventDefault(); $('tCanvas').click(); }
  if (cmd && e.key === '3') { e.preventDefault(); $('tCode').click(); }
  if (cmd && e.key.toLowerCase() === 'z' && document.activeElement !== py) {
    e.preventDefault(); void (async () => { s = await invoke<S>(e.shiftKey ? 'redo' : 'undo'); render(); })();
  }
});

// Scrub anything left behind by the retired custom-caret experiment.
document.querySelectorAll('style[data-aa-caret], .aa-caret').forEach((n) => n.remove());
document.querySelectorAll('math-field').forEach((f) => f.classList.remove('aa-caret-on'));

tools();
void (async () => { s = await invoke<S>('state'); render(); })();
