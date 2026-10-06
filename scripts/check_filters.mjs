// Gate check for ui/filter.js: node scripts/check_filters.mjs
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';

const { filterLibrary, shownTitle, shownDescription } = createRequire(import.meta.url)('../ui/filter.js');
const w = (id, kind, title, extra = {}) => ({ id, kind, added: extra.added ?? 0, info: { title, ...extra.info } });
const lib = [
  w('sea', 'video', 'Sea at dusk', { added: 30, info: { category: 'nature', tags: ['waves', 'calm'] } }),
  w('orbit', 'web', 'Orbit', { added: 10, info: { category: 'space', description: 'Planets turning' } }),
  w('cat', 'gif', 'cat loop', { added: 20, info: { category: 'animals' } }),
  w('yt', 'url', 'Lofi stream', { added: 40 }),
];
const ids = r => r.map(x => x.id);

assert.deepEqual(ids(filterLibrary(lib)), ['cat', 'yt', 'orbit', 'sea'], 'name sort ignores case');
assert.deepEqual(ids(filterLibrary(lib, { sort: 'newest' })), ['yt', 'sea', 'cat', 'orbit']);
assert.deepEqual(ids(filterLibrary(lib, { sort: 'oldest' })), ['orbit', 'cat', 'sea', 'yt']);
assert.deepEqual(ids(filterLibrary(lib, { type: 'video' })), ['sea']);
assert.deepEqual(ids(filterLibrary(lib, { category: 'space' })), ['orbit']);
assert.deepEqual(ids(filterLibrary(lib, { category: 'none' })), ['yt'], 'no category');
assert.deepEqual(ids(filterLibrary(lib, { query: 'WAVES' })), ['sea'], 'tags, any case');
assert.deepEqual(ids(filterLibrary(lib, { query: 'planets' })), ['orbit'], 'description');
assert.deepEqual(ids(filterLibrary(lib, { query: 'sea dusk' })), ['sea'], 'every word must match');
assert.deepEqual(ids(filterLibrary(lib, { query: 'sea orbit' })), [], 'not any word');
const ar = { nature: 'طبيعة', space: 'فضاء', animals: 'حيوانات' };
assert.deepEqual(ids(filterLibrary(lib, { query: 'طبيعة', label: k => ar[k] })), ['sea'], 'category by its shown name');
assert.deepEqual(ids(filterLibrary(lib, { type: 'gif', category: 'nature' })), [], 'filters combine');
assert.equal(lib[0].id, 'sea', 'the input list is left in its order');

// Translated titles: shown in the window's language, found by search, used for sorting.
const intl = [
  w('a', 'video', 'Zebra', { info: { titles: { ar: 'أ حمار وحشي' } } }),
  w('b', 'video', 'Apple', { info: { titles: { ar: 'ب تفاحة' }, descriptions: { ar: 'وصف' }, description: 'desc' } }),
];
assert.equal(shownTitle(intl[0], 'ar'), 'أ حمار وحشي');
assert.equal(shownTitle(intl[0], 'fr'), 'Zebra', 'no French title: the default');
assert.equal(shownTitle(intl[0], ''), 'Zebra');
assert.equal(shownDescription(intl[1], 'ar'), 'وصف');
assert.equal(shownDescription(intl[1], 'de'), 'desc');
assert.deepEqual(ids(filterLibrary(intl, { query: 'تفاحة' })), ['b'], 'search finds a translated title');
assert.deepEqual(ids(filterLibrary(intl, { lang: 'ar' })), ['a', 'b'], 'sorted by the Arabic titles');
assert.deepEqual(ids(filterLibrary(intl, { lang: 'en' })), ['b', 'a'], 'sorted by the English titles');
console.log('filters ok');
