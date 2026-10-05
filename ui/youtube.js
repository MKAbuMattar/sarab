// Plays a YouTube video or playlist. YouTube refuses an embed whose request names no embedding
// site (Error 153), and a wallpaper that loads youtube.com/embed directly names none. This page,
// on Sarab's own origin, frames the player so the request carries that origin.
const q = new URLSearchParams(location.search);
const valid = s => s && /^[\w-]{1,64}$/.test(s);
const id = valid(q.get('v')) ? q.get('v') : null;
const list = valid(q.get('list')) ? q.get('list') : null;

const params = new URLSearchParams({
  autoplay: 1, mute: 1, controls: 0, playsinline: 1, loop: 1, rel: 0,
  enablejsapi: 1, origin: location.origin,
});
// A single video loops only when it is also its own playlist.
if (list) params.set('list', list); else params.set('playlist', id);

const frame = document.createElement('iframe');
frame.allow = 'autoplay; encrypted-media';
frame.referrerPolicy = 'strict-origin-when-cross-origin';
frame.src = `https://www.youtube.com/embed/${id ?? 'videoseries'}?${params}`;
document.body.appendChild(frame);

// Sarab's injected script cannot reach a video inside YouTube's frame, so pause, play and
// volume go through the player's postMessage API. The player ignores commands until it has
// loaded, so the wanted state is kept and sent again once it has.
let paused = false, volume = window.__sarabVolume || 0;
const send = (func, args = []) =>
  frame.contentWindow?.postMessage(JSON.stringify({ event: 'command', func, args }), 'https://www.youtube.com');
const apply = () => {
  send(paused ? 'pauseVideo' : 'playVideo');
  if (volume === 0) send('mute'); else { send('unMute'); send('setVolume', [volume]); }
};
frame.addEventListener('load', () => setTimeout(apply, 1500));

// The player reports its time only after it is told someone listens. Between reports the
// time is estimated from the clock, so it is good to about a tenth of a second, and YouTube
// offers no fine speed steps, so a follower can only jump.
const now = () => performance.timeOrigin + performance.now();
let info = null, infoAt = 0;
frame.addEventListener('load', () =>
  frame.contentWindow?.postMessage(JSON.stringify({ event: 'listening', id: 1, channel: 'widget' }), 'https://www.youtube.com'));
window.addEventListener('message', e => {
  if (e.origin !== 'https://www.youtube.com' || typeof e.data !== 'string') return;
  let m;
  try { m = JSON.parse(e.data); } catch { return; }
  if (m.event === 'infoDelivery' && m.info) {
    info = { ...info, ...m.info };
    if ('currentTime' in m.info) infoAt = now();
  }
});
const playing = () => info && info.playerState === 1 && typeof info.currentTime === 'number';
const current = () => info.currentTime + (now() - infoAt) / 1000;

window.__sarabHooks = {
  freeze() { paused = true; apply(); },
  unfreeze() { paused = false; apply(); },
  volume(v) { volume = v; apply(); },
  time() { return playing() ? { t: current(), at: now(), d: info.duration || 0 } : null; },
  follow(lead) {
    if (!lead || !playing()) return;
    let target = lead.t + (now() - lead.at) / 1000;
    if (lead.d) target %= lead.d;
    let drift = current() - target;
    if (lead.d) drift = ((drift % lead.d) + lead.d * 1.5) % lead.d - lead.d / 2;
    if (Math.abs(drift) > 0.25) send('seekTo', [target, true]);
  },
};
