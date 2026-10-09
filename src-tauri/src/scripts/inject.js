(() => {
  if (window.__sarab) return;
  const nativeRaf = window.requestAnimationFrame.bind(window);
  let fps = 30, frozen = false, last = 0, scheduled = false, nextId = 0, frames = 0;
  let pending = new Map();
  const resumeMedia = new Set();
  let style = null;

  function schedule() {
    if (scheduled || frozen || pending.size === 0) return;
    scheduled = true;
    nativeRaf(tick);
  }
  function tick(t) {
    scheduled = false;
    if (frozen) return;
    const gap = fps > 0 ? 1000 / fps : 0;
    if (gap && t - last < gap - 2) {
      scheduled = true;
      setTimeout(() => { scheduled = false; schedule(); }, gap - (t - last));
      return;
    }
    last = t;
    frames++;
    const run = pending;
    pending = new Map();
    run.forEach(cb => { try { cb(t); } catch (e) { console.error(e); } });
  }
  window.requestAnimationFrame = cb => { const id = ++nextId; pending.set(id, cb); schedule(); return id; };
  window.cancelAnimationFrame = id => { pending.delete(id); };

  const media = () => document.querySelectorAll('video, audio');
  const now = () => performance.timeOrigin + performance.now();

  window.__sarab = {
    setFps(n) { fps = Math.max(0, n | 0); },
    freeze() {
      if (frozen) return;
      frozen = true;
      media().forEach(m => { if (!m.paused) { resumeMedia.add(m); m.pause(); } });
      style = document.createElement('style');
      style.textContent = '*,*::before,*::after{animation-play-state:paused!important}';
      (document.head || document.documentElement).appendChild(style);
      window.__sarabHooks?.freeze?.();
      window.wallpaperPropertyListener?.setPaused?.(true);
    },
    unfreeze() {
      if (!frozen) return;
      frozen = false;
      if (style) { style.remove(); style = null; }
      resumeMedia.forEach(m => m.play().catch(() => {}));
      resumeMedia.clear();
      window.__sarabHooks?.unfreeze?.();
      window.wallpaperPropertyListener?.setPaused?.(false);
      schedule();
    },
    volume(v) {
      media().forEach(m => { m.volume = Math.min(1, v / 100); m.muted = v === 0; });
      window.__sarabVolume = v;
      window.__sarabHooks?.volume?.(v);
    },
    time() {
      const v = document.querySelector('video');
      const hook = window.__sarabHooks?.time?.();
      if (hook !== undefined) return hook;
      if (!v || v.paused) return null;
      v.playbackRate = 1;
      return { t: v.currentTime, at: now(), d: v.duration || 0 };
    },
    follow(lead) {
      if (window.__sarabHooks?.follow) return window.__sarabHooks.follow(lead);
      const v = document.querySelector('video');
      if (!v || !lead || v.paused) return;
      let target = lead.t + (now() - lead.at) / 1000;
      if (lead.d) target %= lead.d;
      let drift = v.currentTime - target;
      if (lead.d) drift = ((drift % lead.d) + lead.d * 1.5) % lead.d - lead.d / 2;
      if (Math.abs(drift) > 0.5) {
        v.currentTime = target;
        v.playbackRate = 1;
      } else {
        v.playbackRate = Math.abs(drift) < 0.002 ? 1 : Math.min(1.1, Math.max(0.9, 1 - drift));
      }
    },
    status() {
      return {
        frozen, fps, frames, hidden: document.hidden, clock: Date.now(),
        media: [...media()].map(m => ({ paused: m.paused, time: m.currentTime, ready: m.readyState })),
        images: [...document.images].map(i => ({ complete: i.complete, width: i.naturalWidth })),
        page: typeof window.sarabStatus === 'function' ? window.sarabStatus() : null,
      };
    },
  };
})();

(() => {
  let listen = null;
  window.wallpaperRegisterAudioListener = f => { listen = typeof f === 'function' ? f : null; };
  window.sarabAudio = levels => {
    if (!listen) return;
    const half = [];
    for (let i = 0; i < 64; i++) half.push((levels[2 * i] + levels[2 * i + 1]) / 2);
    try { listen(half.concat(half)); } catch (e) { console.error(e); }
  };
  window.sarabPropertyChanged = (name, value, page) =>
    window.wallpaperPropertyListener?.applyUserProperties?.({ [name]: { value: page ?? value } });
})();
