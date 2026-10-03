// Injected into every wallpaper page before its own scripts run.
// requestAnimationFrame is wrapped so the host can cap the frame rate and freeze the page.
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
      // Too early: sleep instead of spinning on every vsync.
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
    },
    unfreeze() {
      if (!frozen) return;
      frozen = false;
      if (style) { style.remove(); style = null; }
      resumeMedia.forEach(m => m.play().catch(() => {}));
      resumeMedia.clear();
      window.__sarabHooks?.unfreeze?.();
      schedule();
    },
    volume(v) {
      media().forEach(m => { m.volume = Math.min(1, v / 100); m.muted = v === 0; });
      window.__sarabVolume = v;
    },
    // Video sync between displays. Both sides read the same machine clock, so the follower
    // can add the time the message spent in flight.
    time() {
      const v = document.querySelector('video');
      return v && !v.paused ? { t: v.currentTime, at: Date.now(), d: v.duration || 0 } : null;
    },
    follow(lead) {
      const v = document.querySelector('video');
      if (!v || !lead || v.paused) return;
      let target = lead.t + (Date.now() - lead.at) / 1000;
      if (lead.d) target %= lead.d;
      let drift = v.currentTime - target;
      if (lead.d) drift = ((drift % lead.d) + lead.d * 1.5) % lead.d - lead.d / 2;  // shortest way round the loop
      if (Math.abs(drift) > 0.05) v.currentTime = target;
    },
    status() {
      return {
        frozen, fps, frames, hidden: document.hidden, clock: Date.now(),
        media: [...media()].map(m => ({ paused: m.paused, time: m.currentTime, ready: m.readyState })),
        images: [...document.images].map(i => ({ complete: i.complete, width: i.naturalWidth })),
        // A page may describe itself for diagnostics.
        page: typeof window.sarabStatus === 'function' ? window.sarabStatus() : null,
      };
    },
  };
})();
