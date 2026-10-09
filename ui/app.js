const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = s => document.querySelector(s);

let strings = {};
let state = null;
let selected = null;

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
    const host = sel.closest('dialog') ?? document.body;
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

function ask({ title, body, ok, cancel = null, danger = false }) {
  const d = $('#ask'), okBtn = $('#ask-ok'), cancelBtn = $('#ask-cancel');
  $('#ask-title').textContent = title;
  $('#ask-body').textContent = body;
  okBtn.textContent = ok;
  okBtn.className = danger ? 'danger-fill' : 'accent';
  cancelBtn.hidden = !cancel;
  cancelBtn.textContent = cancel ?? '';
  d.returnValue = 'cancel';
  d.showModal();
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

const lang = () => state?.settings?.language || 'en';
const wpTitle = w => shownTitle(w, lang());
const title = id => { const w = state.library.find(w => w.id === id); return w ? wpTitle(w) : id; };
const kindOf = id => state.library.find(w => w.id === id)?.kind;
const selectedDisplay = () => selected;

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
    const thumb = d.wallpaper && state.library.find(w => w.id === d.wallpaper)?.thumb_url;
    const b = el('button', {
      type: 'button', class: thumb ? 'monitor has-thumb' : 'monitor', 'aria-pressed': String(selected === i),
      'aria-label': `${t('displays.n', { n: i + 1 })}: ${d.wallpaper ? title(d.wallpaper) : t('displays.empty')}, ${t(`reason.${d.reason}`)}`,
      onclick: () => { selected = selected === i ? null : i; render(); },
    },
      el('span', { class: 'num' }, String(i + 1)),
      thumb ? '' : icon(kind ? ICON[kind] ?? '' : ''),
      d.wallpaper ? el('span', { class: `badge ${d.state}` }, t(`state.${d.state}`)) : '',
      el('span', { class: 'label' }, d.wallpaper ? title(d.wallpaper) : t('displays.empty')));
    const slot = order.indexOf(i);
    Object.assign(b.style, {
      left: `${offX + (d.x - minX) * scale + slot * gap}px`, top: `${(d.y - minY) * scale}px`,
      width: `${d.width * scale}px`, height: `${d.height * scale}px`,
    });
    if (thumb) b.style.backgroundImage = `linear-gradient(rgba(0,0,0,.15), rgba(0,0,0,.6)), url(${JSON.stringify(thumb)})`;
    return b;
  }));
}

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

  renderLibrary();

  const f = $('#settings');
  const s = state.settings;
  for (const k of ['pause_fullscreen', 'per_display', 'pause_focus', 'pause_battery', 'pause_power_saver', 'pause_remote']) f[k].checked = s[k];
  f.per_display.disabled = !s.pause_fullscreen;
  f.fps.value = String(s.fps);
  f.pause_cpu.value = String(s.pause_cpu || 0);
  f.audio_mute_others.checked = s.audio_mute_others;
  f.audio_desktop_only.checked = s.audio_desktop_only;
  f.unload_minutes.value = String(s.unload_minutes ?? 0);
  f.scaling.value = s.scaling || 'cover';
  f.screensaver_minutes.value = String(s.screensaver_minutes || 0);
  f.screensaver_wallpaper.replaceChildren(el('option', { value: '' }, t('screensaver.each')),
    ...state.library.filter(w => w.kind !== 'picture').map(w => el('option', { value: w.id }, wpTitle(w))));
  f.screensaver_wallpaper.value = s.screensaver_wallpaper || '';
  f.screensaver_wallpaper.disabled = !s.screensaver_minutes;
  f.span.checked = s.span;
  f.mouse_input.checked = s.mouse_input;
  f.keep_frame_on_quit.checked = s.keep_frame_on_quit;
  f.cycle_minutes.value = String(s.cycle_minutes || 0);
  f.cycle_order.value = s.cycle_order || 'order';
  f.cycle_category.replaceChildren(el('option', { value: 'all' }, t('library.allCategories')),
    ...state.categories.map(c => el('option', { value: c }, categoryName(c))));
  f.cycle_category.value = s.cycle_category || 'all';
  f.cycle_order.disabled = f.cycle_category.disabled = !s.cycle_minutes;
  f.volume.value = s.volume;
  fill(f.volume);
  f.theme.value = s.theme || 'system';
  f.backdrop.value = s.backdrop || 'acrylic';
  f.language.value = s.language;
  if (document.activeElement !== f.app_pause) f.app_pause.value = s.app_pause.join('\n');
  if (document.activeElement !== f.app_play) f.app_play.value = s.app_play.join('\n');
  $('#autostart').checked = state.autostart;
  $('#check-updates').checked = s.check_updates;
  $('#update-channel').value = s.update_channel === 'beta' ? 'beta' : 'stable';
  $('#about-version').textContent = t('about.version', { v: state.version });
  $('#about-webview').textContent = state.webview;
  $('#library-dir').textContent = state.library_dir;
  document.querySelectorAll('select').forEach(combo);
}

const filters = (() => { try { return JSON.parse(localStorage.getItem('filters')) || {}; } catch { return {}; } })();
const saveFilters = () => { try { localStorage.setItem('filters', JSON.stringify(filters)); } catch {} };
const categoryName = c => t(`category.${c}`);

function renderLibrary() {
  const lib = state.library;
  const kinds = [...new Set(lib.map(w => w.kind))];
  if (filters.type && filters.type !== 'all' && !kinds.includes(filters.type)) filters.type = 'all';
  const chip = (value, label) => el('button', {
    type: 'button', 'aria-pressed': String((filters.type || 'all') === value),
    onclick: () => { filters.type = value; saveFilters(); renderLibrary(); },
  }, label);
  $('#lib-types').replaceChildren(chip('all', t('filter.all')), ...kinds.map(k => chip(k, t(`type.${k}`))));

  const cat = $('#lib-category');
  const used = [...new Set(lib.map(w => w.info.category || 'none'))];
  cat.replaceChildren(el('option', { value: 'all' }, t('library.allCategories')),
    ...state.categories.filter(c => used.includes(c)).map(c => el('option', { value: c }, categoryName(c))),
    ...(used.includes('none') ? [el('option', { value: 'none' }, t('edit.none'))] : []));
  cat.value = [...cat.options].some(o => o.value === filters.category) ? filters.category : 'all';
  $('#lib-sort').value = filters.sort || 'name';
  if (document.activeElement !== $('#lib-search')) $('#lib-search').value = filters.query || '';

  const shown = filterLibrary(lib, { type: filters.type || 'all', category: cat.value, query: filters.query || '', sort: filters.sort || 'name', label: categoryName, lang: lang() });
  $('#lib-empty').hidden = lib.length > 0;
  $('#lib-none').hidden = lib.length === 0 || shown.length > 0;
  $('.lib-bar').hidden = $('#lib-types').hidden = lib.length === 0;
  $('#library').replaceChildren(...shown.map(w => el('li', { class: 'tile' },
    thumbFor(w),
    el('div', { class: 'body' },
      el('span', { class: 'name', title: wpTitle(w) }, wpTitle(w)),
      el('span', { class: 'caption line' }, [t(`type.${w.kind}`), w.info.category && categoryName(w.info.category), w.preset && t('presets.builtIn')].filter(Boolean).join(' · ')),
      w.too_new ? el('span', { class: 'caption danger-text' }, t('library.tooNew', { v: w.info.app_version })) : '',
      el('div', { class: 'actions' },
        btn('accent', '', t('library.set'), async () => {
          if (w.too_new && !await ask({ title: t('library.tooNewTitle'), body: t('library.tooNew', { v: w.info.app_version }), ok: t('library.set'), cancel: t('dialog.cancel') })) return;
          if (w.kind === 'app' && !await ask({ title: t('library.appTitle', { title: wpTitle(w) }), body: t('library.appBody'), ok: t('library.run'), cancel: t('dialog.cancel'), danger: true })) return;
          run(() => invoke('set_wallpaper', { target: w.id, display: selectedDisplay() }));
        }),
        el('span', { class: 'spacer' }),
        iconBtn('', t('library.info'), () => openInfo(w)),
        w.preset ? '' : iconBtn('', t('library.edit'), () => openEdit(w)),
        w.preset ? '' : iconBtn('', t('library.delete'), async () => {
          const ok = await ask({ title: t('library.deleteTitle', { title: wpTitle(w) }), body: t('library.deleteBody'), ok: t('library.delete'), cancel: t('dialog.cancel'), danger: true });
          if (ok) run(() => invoke('remove', { id: w.id }));
        }, 'danger'))))));
  document.querySelectorAll('.lib-bar select').forEach(combo);
}

const lessMotion = matchMedia('(prefers-reduced-motion: reduce)');
function thumbFor(w) {
  const still = () => w.thumb_url ? el('img', { src: w.thumb_url, alt: '', loading: 'lazy' }) : icon(ICON[w.kind] ?? '');
  const box = el('div', { class: 'thumb' }, still());
  if (!w.preview_url) return box;
  box.addEventListener('pointerenter', () => {
    if (lessMotion.matches || box.querySelector('.preview')) return;
    const gif = w.kind === 'gif';
    const media = gif
      ? el('img', { class: 'preview', src: w.preview_url, alt: '' })
      : Object.assign(el('video', { class: 'preview', src: w.preview_url }), { muted: true, autoplay: true, loop: true, playsInline: true });
    media.addEventListener(gif ? 'load' : 'playing', () => media.classList.add('on'), { once: true });
    box.append(media);
  });
  box.addEventListener('pointerleave', () => box.querySelector('.preview')?.remove());
  return box;
}

function closeMenu() {
  const m = $('#menu');
  if (m.hidden) return;
  m.hidden = true;
  m.replaceChildren();
  menuReturn?.focus({ preventScroll: true });
}
let menuReturn = null;
function openMenu(x, y, buttons, keyboard) {
  const m = $('#menu');
  m.replaceChildren(...buttons.map(b => el('button', {
    type: 'button', role: 'menuitem', class: b.classList.contains('danger') ? 'danger' : '',
    onclick: () => { closeMenu(); b.click(); },
  }, icon(b.querySelector('.icon')?.textContent ?? ''), el('span', {}, b.getAttribute('aria-label') || b.querySelector('span')?.textContent || ''))));
  m.hidden = false;
  const r = m.getBoundingClientRect();
  const left = document.documentElement.dir === 'rtl' ? (x - r.width < 4 ? x : x - r.width) : (x + r.width > innerWidth ? x - r.width : x);
  m.style.left = `${Math.max(4, left)}px`;
  m.style.top = `${Math.max(4, y + r.height > innerHeight ? y - r.height : y)}px`;
  (keyboard ? m.querySelector('button') : m).focus({ preventScroll: true });
}
document.addEventListener('contextmenu', e => {
  if (e.target.closest('input, textarea')) return;
  e.preventDefault();
  const tile = e.target.closest('.tile');
  const buttons = tile ? [...tile.querySelectorAll('.actions button, .row button')] : [];
  if (!buttons.length) return closeMenu();
  menuReturn = document.activeElement;
  const keyboard = !e.clientX && !e.clientY;
  const at = keyboard ? (r => [r.left, r.bottom])(tile.querySelector('.body').getBoundingClientRect()) : [e.clientX, e.clientY];
  openMenu(...at, buttons, keyboard);
});
document.addEventListener('pointerdown', e => { if (!e.target.closest('#menu')) closeMenu(); });
addEventListener('blur', closeMenu);
addEventListener('resize', closeMenu);
document.addEventListener('scroll', closeMenu, true);
$('#menu').addEventListener('keydown', e => {
  const items = [...$('#menu').querySelectorAll('button')];
  const i = items.indexOf(document.activeElement);
  const go = n => { e.preventDefault(); items[(i + n + items.length) % items.length].focus(); };
  if (e.key === 'ArrowDown') go(1);
  else if (e.key === 'ArrowUp') go(-1);
  else if (e.key === 'Home') go(-i);
  else if (e.key === 'End') go(items.length - 1 - i);
  else if (e.key === 'Escape' || e.key === 'Tab') { e.preventDefault(); closeMenu(); }
});

document.addEventListener('keydown', e => {
  const k = e.key.toLowerCase();
  const browser = ['F5', 'F7', 'BrowserBack', 'BrowserForward', 'BrowserRefresh'].includes(e.key)
    || (e.ctrlKey && ['r', 'p', 's', 'u', 'f', 'g', 'j', 'h'].includes(k))
    || (e.altKey && ['ArrowLeft', 'ArrowRight', 'Home'].includes(e.key));
  const selectAll = e.ctrlKey && k === 'a' && !e.target.closest?.('input, textarea, [contenteditable]');
  if (browser || selectAll) e.preventDefault();
}, true);

const iconBtn = (glyph, label, onclick, cls = '') =>
  el('button', { class: `subtle icon-only ${cls}`, type: 'button', 'aria-label': label, title: label, onclick }, icon(glyph));

function openEdit(w) {
  const d = $('#edit'), f = $('#edit-form');
  f.title.value = w.info.title || '';
  f.description.value = w.info.description || '';
  f.author.value = w.info.author || '';
  f.category.replaceChildren(el('option', { value: '' }, t('edit.none')), ...state.categories.map(c => el('option', { value: c }, categoryName(c))));
  f.category.value = w.info.category || '';
  f.tags.value = (w.info.tags || []).join(', ');
  $('#edit-error').hidden = true;
  combo(f.category);
  d.returnValue = 'cancel';
  d.showModal();
  f.title.focus();
  f.onsubmit = async e => {
    if (e.submitter?.value !== 'save') return;
    e.preventDefault();
    try {
      await invoke('edit_info', { id: w.id, edit: {
        title: f.title.value, description: f.description.value, author: f.author.value,
        category: f.category.value || null, tags: f.tags.value.split(',').map(x => x.trim()).filter(Boolean),
      } });
      d.close('save');
      await refresh();
    } catch (err) {
      $('#edit-error').textContent = String(err);
      $('#edit-error').hidden = false;
    }
  };
}

async function openInfo(w) {
  let det = {};
  try { det = await invoke('details', { id: w.id }); } catch (e) { showError(String(e)); return; }
  const mb = n => n >= 1e6 ? `${(n / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1e3))} KB`;
  const date = s => s ? new Date(s * 1000).toLocaleString(document.documentElement.lang) : t('info.unknown');
  const rows = [
    ['info.type', t(`type.${w.kind}`)],
    ['edit.category', w.info.category ? categoryName(w.info.category) : t('edit.none')],
    ['edit.tags', (w.info.tags || []).join(', ') || t('info.unknown')],
    ['edit.author', w.info.author || t('info.unknown')],
    ['info.license', w.info.license || t('info.unknown')],
    ['info.source', det.source || t('info.unknown')],
    ['info.size', `${mb(det.size || 0)} · ${t('info.files', { n: det.files || 0 })}`],
    ['info.added', date(det.created)],
    ['info.modified', date(det.modified)],
    ['info.version', String(w.info.version || 1)],
    ['info.customize', det.has_props ? t('info.yes') : t('info.no')],
  ];
  $('#info-title').textContent = wpTitle(w);
  $('#info-desc').textContent = shownDescription(w, lang());
  $('#info-desc').hidden = !shownDescription(w, lang());
  $('#info-list').replaceChildren(...rows.flatMap(([k, v]) => [el('dt', {}, t(k)), el('dd', {}, v)]));
  $('#info-folder').onclick = () => run(() => invoke('reveal', { id: w.id }));
  $('#info-export').onclick = () => run(() => invoke('export_wallpaper', { id: w.id }));
  $('#info').showModal();
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
  $('#props-reset').hidden = !entries.length;
  $('#props-reset').onclick = async () => {
    try { await invoke('reset_props', { display }); } catch (e) { showError(String(e)); return; }
    $('#props').close();
    openProps(display);
  };
  if (!$('#props').open) $('#props').showModal();
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
    case 'number': {
      const input = el('input', { type: 'number', min: c.min, max: c.max, step: c.step ?? 1, value: c.value ?? 0 });
      input.addEventListener('change', () => send(input.value));
      return el('label', {}, label, input);
    }
    case 'password': {
      const input = el('input', { type: 'password', value: c.value ?? '', autocomplete: 'off' });
      input.addEventListener('change', () => send(input.value));
      return el('label', {}, label, input);
    }
    case 'button':
      return el('div', {}, el('button', { type: 'button', onclick: () => send(true) }, c.value || label));
    case 'label':
      return el('p', { class: 'caption' }, c.value || label);
    default: {
      const input = el('input', { type: 'text', value: c.value ?? '' });
      input.addEventListener('change', () => send(input.value));
      return el('label', {}, label, input);
    }
  }
}

function fill(r) { r.style.setProperty('--fill', `${((r.value - r.min) / (r.max - r.min || 1)) * 100}%`); }
document.addEventListener('input', e => { if (e.target.type === 'range') fill(e.target); });

let updateLater = false;
let updateAsked = null;

async function askUpdate(u) {
  updateAsked = u.available.version;
  const plain = (u.available.notes || '').replace(/^#+\s*/gm, '').trim();
  const notes = plain ? `${t('update.toastBody', { v: u.available.version })}

${plain}` : t('update.toastBody', { v: u.available.version });
  const ok = await ask({ title: t('update.toastTitle', { v: u.available.version }), body: notes, ok: t('update.now'), cancel: t('update.later') });
  if (ok) run(() => invoke('install_update'));
  else { updateLater = true; render(); }
}

function renderUpdate() {
  const u = state.update;
  if (u.available && u.progress == null && updateAsked !== u.available.version && !$('#ask').open) askUpdate(u);
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
    pause_cpu: Number(f.pause_cpu.value),
    audio_mute_others: f.audio_mute_others.checked,
    audio_desktop_only: f.audio_desktop_only.checked,
    unload_minutes: Number(f.unload_minutes.value),
    scaling: f.scaling.value,
    screensaver_minutes: Number(f.screensaver_minutes.value),
    screensaver_wallpaper: f.screensaver_wallpaper.value || null,
    span: f.span.checked,
    mouse_input: f.mouse_input.checked,
    keep_frame_on_quit: f.keep_frame_on_quit.checked,
    cycle_minutes: Number(f.cycle_minutes.value),
    cycle_order: f.cycle_order.value,
    cycle_category: f.cycle_category.value,
    volume: Number(f.volume.value),
    app_pause: lines(f.app_pause.value),
    app_play: lines(f.app_play.value),
    theme: f.theme.value,
    backdrop: f.backdrop.value,
    check_updates: $('#check-updates').checked,
    update_channel: $('#update-channel').value,
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
$('#lib-search').addEventListener('input', e => { filters.query = e.target.value; saveFilters(); renderLibrary(); });
$('#lib-category').addEventListener('change', e => { filters.category = e.target.value; saveFilters(); renderLibrary(); });
$('#lib-sort').addEventListener('change', e => { filters.sort = e.target.value; saveFilters(); renderLibrary(); });
$('#check-updates').addEventListener('change', () => run(() => invoke('save_settings', { new: readSettings() })));
$('#move-library').addEventListener('click', () => run(() => invoke('move_library')));
$('#export-logs').addEventListener('click', () => run(() => invoke('export_logs')));
$('#reset-settings').addEventListener('click', async () => {
  const ok = await ask({ title: t('about.resetTitle'), body: t('about.resetBody'), ok: t('about.resetButton'), cancel: t('dialog.cancel'), danger: true });
  if (!ok) return;
  await run(() => invoke('reset_settings'));
  await loadLanguage(state.settings.language || 'en');
  render();
});
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
