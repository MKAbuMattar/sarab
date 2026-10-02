// Plays one video or GIF. The host freezes and resumes it through window.__sarab.
const q = new URLSearchParams(location.search);
const fit = ['cover', 'contain', 'fill', 'none'].includes(q.get('fit')) ? q.get('fit') : 'cover';
const gif = q.get('kind') === 'gif';
const el = document.createElement(gif ? 'img' : 'video');
el.style.objectFit = fit;
if (!gif) {
  Object.assign(el, { autoplay: true, loop: true, playsInline: true });
  el.muted = (window.__sarabVolume || 0) === 0;
}
el.src = q.get('src');
document.body.appendChild(el);

if (gif) {
  // CSS cannot pause an animated <img>. Show a still copy of the current frame instead.
  let still = null;
  window.__sarabHooks = {
    freeze() {
      if (still || !el.complete) return;
      still = document.createElement('canvas');
      still.width = el.naturalWidth; still.height = el.naturalHeight;
      still.getContext('2d').drawImage(el, 0, 0);
      still.style.objectFit = fit;
      el.replaceWith(still);
    },
    unfreeze() { if (still) { still.replaceWith(el); still = null; } },
  };
}
