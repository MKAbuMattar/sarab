const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = s => document.querySelector(s);

let strings = {};
let state = null;
let selected = null;  // display index, or null for all displays

const t = (key, vars = {}) => (strings[key] ?? key).replace(/\{(\w+)\}/g, (_, k) => vars[k] ?? '');
const ICON = { web: '', webaudio: '', url: '', video: '', gif: '', picture: '', app: '' };

async function loadLanguage(lang) {
  strings = await (await fetch(`i18n/${lang}.json`)).json();
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
  document.querySelectorAll('[data-t]').forEach(el => { el.textContent = t(el.dataset.t); });
  document.querySelectorAll('[data-tp]').forEach(el => { el.placeholder = t(el.dataset.tp); });
  document.querySelectorAll('[data-tl]').forEach(el => { el.setAttribute('aria-label', t(el.dataset.tl)); });
  document.querySelectorAll('select').forEach(s => s._combo?.sync());
}

// Fluent ComboBox over a hidden native <select>. WebView2 draws the native popup white in dark
// mode, so the select only holds the value and this draws the button and the list.
function combo(sel) {
  if (sel._combo) return sel._combo.sync();
  sel.classList.add('native');
  const text = el('span', { class: 'combo-text' });
  const button = el('button', { type: 'button', class: 'combo', 'aria-haspopup': 'listbox', 'aria-expanded': 'false' }, text, icon('\uE70D'));
  sel.after(button);
  const sync = () => { text.textContent = sel.selectedOptions[0]?.textContent ?? ''; };
  const choose = i => {
    if (i < 0 || i >= sel.options.length || i === sel.selectedIndex) return;
    sel.selectedIndex = i; sync();
    sel.dispatchEvent(new Event('change', { bubbles: true }));
  };
  button.addEventListener('keydown', e => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); choose(sel.selectedIndex + (e.key === 'ArrowDown' ? 1 : -1)); }
  });
  button.addEventListener('click', () => {
    const host = sel.closest('dialog') ?? document.body;  // a modal dialog sits in the top layer
    const list = el('div', { class: 'flyout', role: 'listbox' });
    const items = [...sel.options].map((o, i) => {
      const it = el('div', { class: 'option', role: 'option', tabindex: '-1', 'aria-selected': String(i === sel.selectedIndex) }, o.textContent);
      it.addEventListener('click', () => { choose(i); close(); });
      return it;
    });
    list.append(...items);
    host.append(list);
    const r = button.getBoundingClientRect();
    list.style.minWidth = `${r.width}px`;
    const h = list.offsetHeight, w = list.offsetWidth;
    list.style.top = `${r.bottom + 4 + h > innerHeight - 8 ? Math.max(8, r.top - h - 4) : r.bottom + 4}px`;
    list.style.left = `${document.documentElement.dir === 'rtl' ? r.right - w : r.left}px`;
    button.setAttribute('aria-expanded', 'true');
    let cur = Math.max(0, sel.selectedIndex);
    items[cur]?.focus();
    const outside = e => { if (!list.contains(e.target) && !button.contains(e.target)) close(); };
    function close() {
      list.remove(); button.setAttribute('aria-expanded', 'false');
      document.removeEventListener('pointerdown', outside, true); button.focus();
    }
    document.addEventListener('pointerdown', outside, true);
    list.addEventListener('keydown', e => {
      if (e.key === 'Escape' || e.key === 'Tab') close();
      else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); cur = Math.max(0, Math.min(items.length - 1, cur + (e.key === 'ArrowDown' ? 1 : -1))); items[cur].focus(); }
      else if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); choose(cur); close(); }
    });
  });
  sel._combo = { sync };
  sync();
  return sel._combo;
}

// Windows 11 ContentDialog in place of the browser's confirm() and alert(), which say
// "tauri.localhost says" and ignore the app's theme and language. Resolves true for OK.
function ask({ title, body, ok, cancel = null, danger = false }) {
  const d = $('#ask'), okBtn = $('#ask-ok'), cancelBtn = $('#ask-cancel');
  $('#ask-title').textContent = title;
  $('#ask-body').textContent = body;
  okBtn.textContent = ok;
  okBtn.className = danger ? 'danger-fill' : 'accent';
  cancelBtn.hidden = !cancel;
  cancelBtn.textContent = cancel ?? '';
  d.returnValue = 'cancel';  // Escape keeps this, so it counts as Cancel
  d.showModal();
  // A destructive action starts on Cancel, so Enter alone never deletes.
  (danger && cancel ? cancelBtn : okBtn).focus();
  return new Promise(resolve => d.addEventListener('close', () => resolve(d.returnValue === 'ok'), { once: true }));
}

function showPage(name) {
  document.querySelectorAll('.page').forEach(p => { p.hidden = p.id !== `page-${name}`; });
  document.querySelectorAll('.nav-item[data-page]').forEach(b => {
    if (b.dataset.page === name) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
  });
  try { localStorage.setItem('page', name); } catch {}
}

function showError(e) {
  $('#error').hidden = !e;
  $('#error-text').textContent = e ? `${t('error.prefix')}: ${e}` : '';
}

async function run(fn) {
  try { showError(null); await fn(); } catch (e) { showError(String(e)); }
  await refresh();
}

function el(tag, attrs = {}, ...children) {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k.startsWith('on')) n.addEventListener(k.slice(2), v); else if (v !== undefined) n.setAttribute(k, v);
  }
  n.append(...children);
  return n;
}
const icon = glyph => el('i', { class: 'icon', 'aria-hidden': 'true' }, glyph);
const btn = (cls, glyph, label, onclick) => el('button', { class: cls, type: 'button', onclick }, icon(glyph), el('span', {}, label));

const title = id => state.library.find(w => w.id === id)?.info?.title ?? id;
const kindOf = id => state.library.find(w => w.id === id)?.kind;
const selectedDisplay = () => selected;

// Lay the monitors out like Settings > System > Display: real arrangement, scaled to fit.
function renderMonitors() {
  const box = $('#monitors');
  const ds = state.displays;
  if (!ds.length) { box.replaceChildren(); return; }
  const minX = Math.min(...ds.map(d => d.x)), minY = Math.min(...ds.map(d => d.y));
  const maxX = Math.max(...ds.map(d => d.x + d.width)), maxY = Math.max(...ds.map(d => d.y + d.height));
  const gap = 8, W = box.clientWidth || 600, H = 170;
  const scale = Math.min((W - gap * (ds.length - 1)) / (maxX - minX), H / (maxY - minY));
  const offX = (W - (maxX - minX) * scale - gap * (ds.length - 1)) / 2;
  const order = [...ds.keys()].sort((a, b) => ds[a].x - ds[b].x);
  box.replaceChildren(...ds.map((d, i) => {
    const kind = d.wallpaper ? kindOf(d.wallpaper) : null;
    const b = el('button', {
      type: 'button', class: 'monitor', 'aria-pressed': String(selected === i),
      'aria-label': `${t('displays.n', { n: i + 1 })}: ${d.wallpaper ? title(d.wallpaper) : t('displays.empty')}, ${t(`reason.${d.reason}`)}`,
      onclick: () => { selected = selected === i ? null : i; render(); },
    },
      el('span', { class: 'num' }, String(i + 1)),
      icon(kind ? ICON[kind] ?? '' : ''),
      d.wallpaper ? el('span', { class: `badge ${d.state}` }, t(`state.${d.state}`)) : '',
      el('span', { class: 'label' }, d.wallpaper ? title(d.wallpaper) : t('displays.empty')));
    const slot = order.indexOf(i);
    Object.assign(b.style, {
      left: `${offX + (d.x - minX) * scale + slot * gap}px`, top: `${(d.y - minY) * scale}px`,
      width: `${d.width * scale}px`, height: `${d.height * scale}px`,
    });
    return b;
  }));
}

// What the selected display is doing and why, with the actions that apply to it.
function renderDetail() {
  const box = $('#detail');
  const idx = selected ?? (state.displays.length === 1 ? 0 : null);
  if (idx === null) {
    const paused = state.displays.filter(d => d.wallpaper && d.state !== 'Play');
    const why = paused.length ? t(`reason.${paused[0].reason}`) : t('detail.pickHint');
    box.replaceChildren(el('span', { class: 'why caption' }, why),
      paused.length ? btn('', '', t('detail.playAnyway'), () => run(() => invoke('play_anyway'))) : '',
      state.manual === false ? btn('', '', t('detail.auto'), () => run(() => invoke('resume_auto'))) : '');
    return;
  }
  const d = state.displays[idx];
  box.replaceChildren(
    el('span', { class: 'why' }, el('strong', {}, t('displays.n', { n: idx + 1 })), ' · ',
      d.wallpaper ? `${title(d.wallpaper)} · ${t(`reason.${d.reason}`)}` : t('displays.empty'),
      d.error ? el('span', { class: 'caption' }, ` · ${t('error.prefix')}: ${d.error}`) : ''),
    d.wallpaper && d.state !== 'Play' ? btn('', '', t('detail.playAnyway'), () => run(() => invoke('play_anyway'))) : '',
    state.manual === false ? btn('', '', t('detail.auto'), () => run(() => invoke('resume_auto'))) : '',
    d.wallpaper ? btn('', '', t('library.customize'), () => openProps(idx)) : '',
    d.wallpaper ? btn('subtle', '', t('displays.close'), () => run(() => invoke('close', { display: idx }))) : '');
}

function render() {
  document.documentElement.dataset.theme = state.theme;
  document.documentElement.classList.toggle('translucent', state.translucent);
  const paused = state.manual === true;
  $('#pause-icon').textContent = paused ? '' : '';
  $('#pause-text').textContent = paused ? t('pause.resume') : t('pause.button');

  if (selected !== null && selected >= state.displays.length) selected = null;
  $('#all-displays').setAttribute('aria-pressed', String(selected === null));
  renderMonitors();
  renderDetail();

  renderUpdate();
  renderPresets();

  $('#lib-empty').hidden = state.library.length > 0;
  $('#library').replaceChildren(...state.library.map(w => el('li', { class: 'tile' },
    el('div', { class: 'thumb' }, icon(ICON[w.kind] ?? '')),
    el('div', { class: 'body' },
      el('span', { class: 'name' }, w.info.title || w.id),
      el('span', { class: 'caption' }, w.preset ? `${t(`type.${w.kind}`)} · ${t('presets.builtIn')}` : t(`type.${w.kind}`)),
      el('div', { class: 'row' },
        btn('accent', '', t('library.set'), () => run(() => invoke('set_wallpaper', { target: w.id, display: selectedDisplay() }))),
        w.preset ? '' : btn('subtle danger', '', t('library.delete'), async () => {
          const ok = await ask({ title: t('library.deleteTitle', { title: w.info.title || w.id }), body: t('library.deleteBody'), ok: t('library.delete'), cancel: t('dialog.cancel'), danger: true });
          if (ok) run(() => invoke('remove', { id: w.id }));
        }))))));

  const f = $('#settings');
  const s = state.settings;
  for (const k of ['pause_fullscreen', 'per_display', 'pause_focus', 'pause_battery', 'pause_power_saver', 'pause_remote']) f[k].checked = s[k];
  f.per_display.disabled = !s.pause_fullscreen;
  f.fps.value = String(s.fps);
  f.volume.value = s.volume;
  fill(f.volume);
  f.theme.value = s.theme || 'system';
  f.backdrop.value = s.backdrop || 'acrylic';
  f.language.value = s.language;
  if (document.activeElement !== f.app_pause) f.app_pause.value = s.app_pause.join('\n');
  if (document.activeElement !== f.app_play) f.app_play.value = s.app_play.join('\n');
  $('#autostart').checked = state.autostart;
  $('#check-updates').checked = s.check_updates;
  $('#about-version').textContent = t('about.version', { v: state.version });
  $('#about-webview').textContent = state.webview;
  document.querySelectorAll('select').forEach(combo);
}

async function refresh() {
  state = await invoke('state');
  render();
}

async function openProps(display) {
  const ctls = await invoke('props', { display });
  const entries = Object.entries(ctls);
  $('#props-body').replaceChildren(...(entries.length
    ? entries.map(([key, c]) => control(display, key, c))
    : [el('p', { class: 'caption' }, t('props.none'))]));
  $('#props-body').querySelectorAll('select').forEach(combo);
  $('#props').showModal();
}

function control(display, key, c) {
  const send = value => invoke('set_prop', { display, key, value: String(value) }).catch(e => showError(String(e)));
  const label = c.text || key;
  switch (c.type) {
    case 'slider': {
      const out = el('output', { class: 'caption' }, c.value);
      const input = el('input', { type: 'range', min: c.min, max: c.max, step: c.step ?? 1, value: c.value });
      requestAnimationFrame(() => fill(input));
      input.addEventListener('input', () => { out.textContent = input.value; send(input.value); });
      return el('label', {}, el('span', { class: 'row' }, label, out), input);
    }
    case 'checkbox': {
      const input = el('input', { type: 'checkbox', class: 'switch' });
      input.checked = !!c.value;
      input.addEventListener('change', () => send(input.checked));
      return el('label', { class: 'switch-label' }, input, el('span', {}, label));
    }
    case 'dropdown':
    case 'scalerDropdown': {
      const input = el('select', {}, ...(c.items || []).map((it, i) => el('option', { value: i }, it)));
      input.value = c.value;
      input.addEventListener('change', () => send(input.value));
      return el('label', {}, label, input);
    }
    case 'color': {
      const input = el('input', { type: 'color', value: c.value });
      input.addEventListener('input', () => send(input.value));
      return el('label', {}, label, input);
    }
    case 'button':
      return el('div', {}, el('button', { type: 'button', onclick: () => send(true) }, c.value || label));
    case 'label':
      return el('p', { class: 'caption' }, c.value || label);
    default: {
      // textbox, and folderDropdown as a plain file name until the folder picker lands.
      const input = el('input', { type: 'text', value: c.value ?? '' });
      input.addEventListener('change', () => send(input.value));
      return el('label', {}, label, input);
    }
  }
}

// The Fluent slider paints its filled part from --fill.
function fill(r) { r.style.setProperty('--fill', `${((r.value - r.min) / (r.max - r.min || 1)) * 100}%`); }
document.addEventListener('input', e => { if (e.target.type === 'range') fill(e.target); });

// Update banner: shown on every page while a newer version exists. "Later" hides it until the
// window is opened again; the check itself repeats once a day.
let updateLater = false;
function renderUpdate() {
  const u = state.update;
  const bar = $('#update-bar');
  bar.hidden = !u.available || (updateLater && u.progress == null);
  if (u.available) {
    $('#update-text').textContent = u.progress != null
      ? t('update.downloading', { v: u.available.version, p: u.progress })
      : t('update.available', { v: u.available.version });
    $('#update-actions').replaceChildren(...(u.progress != null ? [] : [
      btn('accent', '\uE896', t('update.now'), () => run(() => invoke('install_update'))),
      u.available.notes ? btn('subtle', '\uE8A5', t('update.notes'), () => ask({ title: t('update.notesTitle', { v: u.available.version }), body: u.available.notes, ok: t('dialog.close') })) : '',
      btn('subtle', '\uE711', t('update.later'), () => { updateLater = true; render(); }),
    ]));
  }
  $('#update-status').textContent = u.available
    ? t('update.available', { v: u.available.version })
    : u.error ? `${t('update.failed')} ${u.error}`
    : u.checked ? t('update.latest', { v: state.version }) : t('update.never');
}

// NASA 4K videos: downloaded only when asked, and verified against a pinned checksum.
function renderPresets() {
  const mb = n => Math.round(n / 1e6);
  $('#presets').replaceChildren(...state.presets.map(p => el('li', { class: 'tile' },
    el('div', { class: 'thumb' }, icon('\uE714')),
    el('div', { class: 'body' },
      el('span', { class: 'name' }, p.title),
      el('span', { class: 'caption' }, `${p.width}×${p.height} · ${p.fps} fps · ${mb(p.size)} MB`),
      el('span', { class: 'caption' }, p.description),
      el('span', { class: 'caption' }, p.credit),
      el('div', { class: 'row' },
        p.installed
          ? btn('accent', '\uE7F4', t('library.set'), () => run(() => invoke('set_wallpaper', { target: p.id, display: selectedDisplay() })))
          : p.progress != null
            ? el('span', { class: 'caption' }, t('presets.downloading', { p: p.progress }))
            : btn('', '\uE896', t('presets.get', { mb: mb(p.size) }), () => run(() => invoke('get_preset', { id: p.id }))))))));
}

function readSettings() {
  const f = $('#settings');
  const lines = v => v.split('\n').map(x => x.trim()).filter(Boolean);
  return {
    ...state.settings,
    pause_fullscreen: f.pause_fullscreen.checked,
    per_display: f.per_display.checked,
    pause_focus: f.pause_focus.checked,
    pause_battery: f.pause_battery.checked,
    pause_power_saver: f.pause_power_saver.checked,
    pause_remote: f.pause_remote.checked,
    fps: Number(f.fps.value),
    volume: Number(f.volume.value),
    app_pause: lines(f.app_pause.value),
    app_play: lines(f.app_play.value),
    theme: f.theme.value,
    backdrop: f.backdrop.value,
    check_updates: $('#check-updates').checked,
    language: f.language.value,
  };
}

$('#add').addEventListener('submit', e => {
  e.preventDefault();
  const target = $('#target').value.trim();
  run(async () => {
    await invoke('set_wallpaper', { target, display: selectedDisplay() });
    $('#target').value = '';
  });
});
$('#check-updates').addEventListener('change', () => run(() => invoke('save_settings', { new: readSettings() })));
$('#check-now').addEventListener('click', () => run(() => invoke('check_update')));
$('#pause').addEventListener('click', () => run(() => invoke('toggle_pause')));
document.querySelectorAll('[data-open]').forEach(b => b.addEventListener('click', () => run(() => invoke('open', { which: b.dataset.open }))));
$('#all-displays').addEventListener('click', () => { selected = null; render(); });
window.addEventListener('resize', () => state && renderMonitors());
document.querySelectorAll('.nav-item[data-page]').forEach(b => b.addEventListener('click', () => showPage(b.dataset.page)));
$('#settings').addEventListener('change', async e => {
  if (e.target.id === 'autostart') return run(() => invoke('autostart', { enable: e.target.checked }));
  if (e.target.name === 'language') await loadLanguage(e.target.value);
  await run(() => invoke('save_settings', { new: readSettings() }));
});

listen('changed', refresh);

(async () => {
  state = await invoke('state');
  document.documentElement.dataset.theme = state.theme;
  await loadLanguage(state.settings.language || 'en');
  let page = 'library';
  try { page = localStorage.getItem('page') || page; } catch {}
  showPage(['library', 'settings', 'about'].includes(page) ? page : 'library');
  render();
})();
