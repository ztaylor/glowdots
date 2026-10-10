import init, { Demo } from './pkg/glowdots_web.js';

const $ = (id) => document.getElementById(id);

const SCENES = {
  clock: 'Clock', weather: 'Weather', lake: 'Lake', news: 'News board', events: 'Events',
  sports: 'Sports', status: 'Status board', winter: 'Lake in winter', demo: 'Demoscene',
};

const LOOKS = [
  ['scene', "Each scene's own", 'conic-gradient(#ff4305, #3070ff, #30ff40, #ff4305)'],
  ['williams', 'Williams orange', '#ff4305'],
  ['amber', 'Amber', '#ff8014'],
  ['red', 'Red', '#ff140a'],
  ['green', 'Green', '#40ff4d'],
  ['blue', 'Blue', '#4d8cff'],
  ['white', 'White', '#ffebcc'],
  ['heat', 'Heat', 'linear-gradient(90deg, #600800, #d03000, #ff9010, #fff0b0)'],
  ['ocean', 'Ocean', 'linear-gradient(90deg, #001860, #0080c0, #40f0d0, #f0ffff)'],
];

const KNOBS = [
  ['dot_size', 'Dot size', 0.2, 0.5, 0.005],
  ['roundness', 'Roundness', 0, 1, 0.01],
  ['halo', 'Halo', 0, 0.4, 0.005],
  ['halo_width', 'Halo reach', 0.1, 1, 0.01],
  ['unlit', 'Unlit dots', 0, 0.2, 0.005],
  ['variation', 'Unevenness', 0, 0.6, 0.01],
  ['flicker', 'Shimmer', 0, 0.3, 0.005],
  ['gamma', 'Gamma', 0.6, 3, 0.05],
  ['hot_core', 'White-hot centers', 0, 1, 0.01],
  ['bloom', 'Bloom', 0, 2, 0.01],
  ['afterglow', 'Afterglow', 0, 0.3, 0.005],
];

const FEED_SCENE = {
  'events.json': 'events', 'sports.json': 'sports', 'news.json': 'news',
  'status.json': 'status', 'weather.json': 'weather', 'lake.json': 'lake',
};

function fail(message) {
  const fault = $('fault');
  fault.textContent = message;
  fault.hidden = false;
}

// Size the canvas to a whole number of screen pixels per dot, so every
// dot is the same shape and the panel fills the bezel.
const canvas = $('dmd');
const fit = () => {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const room = canvas.parentElement.clientWidth - parseFloat(getComputedStyle(canvas.parentElement).paddingLeft) * 2;
  const pitch = Math.max(1, Math.floor((room * dpr) / 80));
  canvas.style.width = `${(pitch * 80) / dpr}px`;
  canvas.style.height = `${(pitch * 32) / dpr}px`;
};
new ResizeObserver(fit).observe(canvas.parentElement);
fit();

let demo;
try {
  await init();
  try {
    demo = new Demo(canvas);
  } catch (e) {
    console.error(e);
    fail("The display needs WebGL 2, which this browser can't run. Try a current Chrome, Firefox, or Safari.");
  }
} catch (e) {
  console.error(e);
  fail('The display could not load. Serve this folder over HTTP instead of opening the file: python3 -m http.server -d web/www');
}

if (demo) {
  // Scenes.
  const chips = $('scenes');
  for (const name of demo.scenes()) {
    const b = document.createElement('button');
    b.type = 'button';
    b.textContent = SCENES[name] ?? name;
    b.dataset.scene = name;
    b.addEventListener('click', () => demo.show(name));
    chips.append(b);
  }
  let shown = '';
  const follow = () => {
    const now = demo.current();
    if (now !== shown) {
      shown = now;
      for (const b of chips.children) b.setAttribute('aria-current', String(b.dataset.scene === now));
    }
  };
  setInterval(follow, 200);
  follow();
  $('hold').addEventListener('change', (e) => demo.set_hold(e.target.checked));
  for (const b of document.querySelectorAll('[data-show]')) b.addEventListener('click', () => demo.show(b.dataset.show));

  // Glass.
  const sliders = $('sliders');
  const inputs = {};
  for (const [key, label, min, max, step] of KNOBS) {
    const id = `glass-${key}`;
    const l = document.createElement('label');
    l.htmlFor = id;
    l.textContent = label;
    const input = Object.assign(document.createElement('input'), { type: 'range', id, min, max, step });
    const out = document.createElement('output');
    out.htmlFor = id;
    input.addEventListener('input', () => {
      demo.set_glass_value(key, Number(input.value));
      out.textContent = Number(input.value).toFixed(2);
    });
    inputs[key] = [input, out];
    sliders.append(l, input, out);
  }
  const readGlass = () => {
    for (const [key, [input, out]] of Object.entries(inputs)) {
      const v = demo.glass_value(key);
      input.value = v;
      out.textContent = v.toFixed(2);
    }
  };
  readGlass();
  const glass = $('glass');
  glass.addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    demo.set_glass(b.dataset.glass);
    for (const o of glass.children) o.setAttribute('aria-checked', String(o === b));
    readGlass();
  });

  // Color.
  const looks = $('looks');
  for (const [key, label, color] of LOOKS) {
    const b = document.createElement('button');
    b.type = 'button';
    b.setAttribute('role', 'radio');
    b.setAttribute('aria-checked', String(key === 'scene'));
    const dot = document.createElement('i');
    dot.style.setProperty('--c', color);
    if (!color.includes('gradient')) dot.style.setProperty('--glow', color);
    b.append(dot, label);
    b.addEventListener('click', () => {
      demo.set_look(key);
      for (const o of looks.children) o.setAttribute('aria-checked', String(o === b));
    });
    looks.append(b);
  }

  // Time of day.
  const time = $('time');
  const timeOut = $('time-out');
  const clock = (m) => {
    const h = Math.floor(m / 60) % 24;
    return `${(h + 11) % 12 + 1}:${String(m % 60).padStart(2, '0')} ${h < 12 ? 'AM' : 'PM'}`;
  };
  let real = true;
  const showTime = () => {
    if (real) time.value = demo.minutes();
    timeOut.textContent = clock(Number(time.value));
  };
  setInterval(showTime, 1000);
  showTime();
  time.addEventListener('input', () => {
    real = false;
    demo.set_time(Number(time.value));
    timeOut.textContent = clock(Number(time.value));
  });
  $('real-time').addEventListener('click', () => {
    real = true;
    demo.set_real_time();
    showTime();
  });

  // Your data.
  const feed = $('feed');
  const text = $('feed-text');
  const status = $('feed-status');
  const samples = {};
  const say = (message, kind) => {
    status.textContent = message;
    status.className = `status ${kind ?? ''}`;
  };
  const load = () => {
    samples[feed.value] ??= demo.feed(feed.value);
    text.value = demo.feed(feed.value);
    say('');
  };
  feed.addEventListener('change', load);
  load();
  $('feed-show').addEventListener('click', () => {
    const problem = demo.set_feed(feed.value, text.value);
    if (problem) say(`${problem}. The display says so too.`, 'bad');
    else say('Shown on the display.', 'ok');
    demo.show(FEED_SCENE[feed.value]);
  });
  $('feed-reset').addEventListener('click', () => {
    demo.set_feed(feed.value, samples[feed.value]);
    text.value = samples[feed.value];
    say('The sample is back.');
  });
}
