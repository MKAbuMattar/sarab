(() => {
  if (window.__sarab) return;
  const nativeRaf = window.requestAnimationFrame.bind(window);
  let fps = 30,
    frozen = false,
    last = 0,
    scheduled = false,
    nextId = 0,
    frames = 0;
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
      setTimeout(
        () => {
          scheduled = false;
          schedule();
        },
        gap - (t - last),
      );
      return;
    }
    last = t;
    frames++;
    const run = pending;
    pending = new Map();
    run.forEach((cb) => {
      try {
        cb(t);
      } catch (e) {
        console.error(e);
      }
    });
  }
  window.requestAnimationFrame = (cb) => {
    const id = ++nextId;
    pending.set(id, cb);
    schedule();
    return id;
  };
  window.cancelAnimationFrame = (id) => {
    pending.delete(id);
  };

  const media = () => document.querySelectorAll("video, audio");
  const now = () => performance.timeOrigin + performance.now();

  window.__sarab = {
    setFps(n) {
      fps = Math.max(0, n | 0);
    },
    freeze() {
      if (frozen) return;
      frozen = true;
      media().forEach((m) => {
        if (!m.paused) {
          resumeMedia.add(m);
          m.pause();
        }
      });
      style = document.createElement("style");
      style.textContent =
        "*,*::before,*::after{animation-play-state:paused!important}";
      (document.head || document.documentElement).appendChild(style);
      window.__sarabHooks?.freeze?.();
      window.wallpaperPropertyListener?.setPaused?.(true);
    },
    unfreeze() {
      if (!frozen) return;
      frozen = false;
      if (style) {
        style.remove();
        style = null;
      }
      resumeMedia.forEach((m) => m.play().catch(() => {}));
      resumeMedia.clear();
      window.__sarabHooks?.unfreeze?.();
      window.wallpaperPropertyListener?.setPaused?.(false);
      schedule();
    },
    volume(v) {
      media().forEach((m) => {
        m.volume = Math.min(1, v / 100);
        m.muted = v === 0;
      });
      window.__sarabVolume = v;
      window.__sarabHooks?.volume?.(v);
    },
    time() {
      const v = document.querySelector("video");
      const hook = window.__sarabHooks?.time?.();
      if (hook !== undefined) return hook;
      if (!v || v.paused) return null;
      v.playbackRate = 1;
      return { t: v.currentTime, at: now(), d: v.duration || 0 };
    },
    follow(lead) {
      if (window.__sarabHooks?.follow) return window.__sarabHooks.follow(lead);
      const v = document.querySelector("video");
      if (!v || !lead || v.paused) return;
      let target = lead.t + (now() - lead.at) / 1000;
      if (lead.d) target %= lead.d;
      let drift = v.currentTime - target;
      if (lead.d)
        drift = (((drift % lead.d) + lead.d * 1.5) % lead.d) - lead.d / 2;
      if (Math.abs(drift) > 0.5) {
        v.currentTime = target;
        v.playbackRate = 1;
      } else {
        v.playbackRate =
          Math.abs(drift) < 0.002 ? 1 : Math.min(1.1, Math.max(0.9, 1 - drift));
      }
    },
    status() {
      return {
        frozen,
        fps,
        frames,
        hidden: document.hidden,
        clock: Date.now(),
        media: [...media()].map((m) => ({
          paused: m.paused,
          time: m.currentTime,
          ready: m.readyState,
        })),
        images: [...document.images].map((i) => ({
          complete: i.complete,
          width: i.naturalWidth,
        })),
        page:
          typeof window.sarabStatus === "function"
            ? window.sarabStatus()
            : null,
      };
    },
  };
})();

(() => {
  let listen = null;
  window.wallpaperRegisterAudioListener = (f) => {
    listen = typeof f === "function" ? f : null;
  };
  window.sarabAudio = (levels) => {
    if (!listen) return;
    const half = [];
    for (let i = 0; i < 64; i++)
      half.push((levels[2 * i] + levels[2 * i + 1]) / 2);
    try {
      listen(half.concat(half));
    } catch (e) {
      console.error(e);
    }
  };
  window.sarabPropertyChanged = (name, value, page) =>
    window.wallpaperPropertyListener?.applyUserProperties?.({
      [name]: { value: page ?? value },
    });
})();

(() => {
  if (window.wallpaperMediaIntegration) return;
  const on = {
    status: [],
    properties: [],
    thumbnail: [],
    playback: [],
    timeline: [],
  };
  const register = (kind) => (f) => {
    if (typeof f === "function") on[kind].push(f);
  };
  const send = (kind, value) =>
    on[kind].forEach((f) => {
      try {
        f(value);
      } catch (e) {
        console.error(e);
      }
    });
  window.wallpaperMediaIntegration = {
    PLAYBACK_STOPPED: 0,
    PLAYBACK_PLAYING: 1,
    PLAYBACK_PAUSED: 2,
  };
  window.wallpaperRegisterMediaStatusListener = register("status");
  window.wallpaperRegisterMediaPropertiesListener = register("properties");
  window.wallpaperRegisterMediaThumbnailListener = register("thumbnail");
  window.wallpaperRegisterMediaPlaybackListener = register("playback");
  window.wallpaperRegisterMediaTimelineListener = register("timeline");

  const hex = (c) =>
    "#" + c.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
  const colors = (src) =>
    new Promise((done) => {
      const fallback = {
        primaryColor: "#000000",
        secondaryColor: "#000000",
        tertiaryColor: "#000000",
        textColor: "#ffffff",
        highContrastColor: "#ffffff",
      };
      if (!src) return done(fallback);
      const img = new Image();
      img.onerror = () => done(fallback);
      img.onload = () => {
        const c = document.createElement("canvas");
        c.width = c.height = 16;
        const g = c.getContext("2d");
        g.drawImage(img, 0, 0, 16, 16);
        const px = g.getImageData(0, 0, 16, 16).data;
        const buckets = new Map();
        for (let i = 0; i < px.length; i += 4) {
          const k = (px[i] >> 5) * 64 + (px[i + 1] >> 5) * 8 + (px[i + 2] >> 5);
          const b = buckets.get(k) || { n: 0, c: [0, 0, 0] };
          b.n++;
          for (let j = 0; j < 3; j++) b.c[j] += px[i + j];
          buckets.set(k, b);
        }
        const top = [...buckets.values()]
          .sort((a, b) => b.n - a.n)
          .map((b) => b.c.map((v) => v / b.n));
        const pick = (i) => top[Math.min(i, top.length - 1)];
        const [r, gr, b] = pick(0);
        const text = 0.299 * r + 0.587 * gr + 0.114 * b > 150 ? "#000000" : "#ffffff";
        done({
          primaryColor: hex(pick(0)),
          secondaryColor: hex(pick(1)),
          tertiaryColor: hex(pick(2)),
          textColor: text,
          highContrastColor: text,
        });
      };
      img.src = src;
    });

  let last,
    timeline = null;
  const tick = () => {
    if (!timeline) return;
    if (timeline.playing && performance.now() - timeline.sent >= 1000) {
      const position = Math.min(
        timeline.duration || Infinity,
        timeline.position + (performance.now() - timeline.at) / 1000,
      );
      timeline.sent = performance.now();
      send("timeline", { position, duration: timeline.duration });
    }
    requestAnimationFrame(tick);
  };
  window.__sarabMedia = (t) => {
    try {
      if (typeof window.sarabNowPlaying === "function") window.sarabNowPlaying(t);
    } catch (e) {
      console.error(e);
    }
    const prev = last;
    last = t || null;
    if (prev === undefined || !!prev !== !!last)
      send("status", { enabled: !!last });
    if (!last) {
      timeline = null;
      return;
    }
    const song = (x) =>
      x && [x.Title, x.Artist, x.AlbumTitle, x.AlbumArtist].join("\u0000");
    if (song(prev) !== song(last))
      send("properties", {
        title: last.Title,
        artist: last.Artist,
        subTitle: "",
        albumTitle: last.AlbumTitle,
        albumArtist: last.AlbumArtist,
        genres: "",
        contentType: "music",
      });
    if (!prev || prev.Thumbnail !== last.Thumbnail)
      colors(last.Thumbnail).then((c) =>
        send("thumbnail", { thumbnail: last.Thumbnail, ...c }),
      );
    if (!prev || prev.State !== last.State)
      send("playback", {
        state: { playing: 1, paused: 2 }[last.State] ?? 0,
      });
    const running = !!timeline;
    timeline = {
      position: last.Position || 0,
      duration: last.Duration || 0,
      playing: last.State === "playing",
      at: performance.now(),
      sent: performance.now(),
    };
    send("timeline", { position: timeline.position, duration: timeline.duration });
    if (!running) requestAnimationFrame(tick);
  };
})();
