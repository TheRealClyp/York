(function () {
'use strict';
var NL = String.fromCharCode(10);

/* ---------- OS detection ---------- */
function os() {
  var u = (navigator.userAgent || '').toLowerCase();
  if (/windows|win32|win64/.test(u)) return 'windows';
  if (/macintosh|mac os x/.test(u)) return 'macos';
  return 'linux';
}
var OS = os();

/* ---------- platform panes ---------- */
var panes = Array.prototype.slice.call(document.querySelectorAll('.pane[data-os]'));
var osbtns = Array.prototype.slice.call(document.querySelectorAll('.osbtn[data-os]'));
function pickPane(name) {
  panes.forEach(function (p) { p.classList.toggle('active', p.getAttribute('data-os') === name); });
  osbtns.forEach(function (b) { b.classList.toggle('active', b.getAttribute('data-os') === name); });
}
osbtns.forEach(function (b) {
  b.addEventListener('click', function () { pickPane(b.getAttribute('data-os')); });
});
if (OS !== 'windows') { pickPane(OS); }

/* ---------- big download button ---------- */
var dlbtn = document.getElementById('dlbtn');
var dlhint = document.getElementById('dlhint');
if (dlbtn) {
  if (OS === 'linux') {
    dlbtn.href = 'downloads/york-x86_64-linux.tar.gz';
    dlbtn.textContent = 'Download for Linux';
    if (dlhint) { dlhint.textContent = 'Auto-detected: Linux · york-x86_64-linux.tar.gz · SHA-256 verified.'; }
  } else if (OS === 'macos') {
    dlbtn.href = '#install';
    dlbtn.textContent = 'macOS — coming soon';
    if (dlhint) { dlhint.textContent = 'Windows and Linux binaries are ready today. macOS is produced on a Mac and lands here next.'; }
  } else {
    dlbtn.href = 'downloads/york-setup-x64.exe';
    dlbtn.textContent = 'Download for Windows';
    if (dlhint) { dlhint.textContent = 'Auto-detected: Windows · york-setup-x64.exe · SHA-256 verified.'; }
  }
}

/* ---------- code viewer tabs ---------- */
var tabs = Array.prototype.slice.call(document.querySelectorAll('.codewin .tab'));
var snips = Array.prototype.slice.call(document.querySelectorAll('.snipp'));
tabs.forEach(function (t) {
  t.addEventListener('click', function () {
    tabs.forEach(function (x) { x.classList.remove('active'); });
    snips.forEach(function (s) { s.classList.remove('active'); });
    t.classList.add('active');
    var target = t.getAttribute('data-snip');
    snips.forEach(function (s) { if (s.getAttribute('data-snip') === target) { s.classList.add('active'); } });
  });
});

/* ---------- copy buttons ---------- */
function copyText(text, btn) {
  var done = function () {
    var old = btn.textContent;
    btn.textContent = 'copied!';
    btn.classList.add('done');
    setTimeout(function () { btn.textContent = old; btn.classList.remove('done'); }, 1400);
  };
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).then(done, function () { fallback(text, btn, done); });
  } else { fallback(text, btn, done); }
}
function fallback(text, btn, done) {
  var ta = document.createElement('textarea');
  ta.value = text;
  ta.style.position = 'fixed';
  ta.style.opacity = '0';
  document.body.appendChild(ta);
  ta.select();
  try { document.execCommand('copy'); done(); } catch (e) { btn.textContent = 'press ctrl+c'; }
  document.body.removeChild(ta);
}
Array.prototype.slice.call(document.querySelectorAll('.copy')).forEach(function (b) {
  b.addEventListener('click', function () {
    var pre = b.parentElement.querySelector('pre');
    copyText(pre.textContent.trim(), b);
  });
});

/* ---------- hero typing demo ---------- */
var hero = document.getElementById('heroterm');
if (hero) {
  var tape = [
    'public static void main(String[] args) {',
    '    Arena<Player> players = new Arena(1000);',
    '    players.push(Player { pos: 1.0, hp: 100.0 });',
    '',
    '    for (Player p : players) {',
    '        p.damage(10.0);',
    '        println(p.hp);',
    '    }',
    '}'
  ];
  var cursor = document.createElement('span');
  cursor.className = 'cursor';
  var li = 0, ci = 0;
  var tick = 13;
  var move = true;
  if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) { move = false; }
  function typeLine() {
    if (li >= tape.length) { showRun(); return; }
    var line = tape[li];
    if (ci < line.length) {
      hero.appendChild(document.createTextNode(line.charAt(ci)));
      ci++;
      if (move) { setTimeout(typeLine, tick); } else { while (ci < line.length) { hero.appendChild(document.createTextNode(line.charAt(ci))); ci++; } typeLine(); }
    } else {
      hero.appendChild(document.createTextNode(NL));
      li++; ci = 0;
      if (move) { setTimeout(typeLine, 130); } else { typeLine(); }
    }
  }
  function showRun() {
    hero.appendChild(document.createTextNode(NL));
    var d = document.createElement('span');
    d.className = 'dim';
    d.textContent = '$ york run hello.yk';
    hero.appendChild(d);
    hero.appendChild(document.createTextNode(NL));
    var o = document.createElement('span');
    o.className = 'out';
    o.textContent = 'Hello, York!';
    hero.appendChild(o);
  }
  hero.appendChild(cursor);
  window.setTimeout(typeLine, 350);
}

/* ---------- reveal on scroll ---------- */
var secs = Array.prototype.slice.call(document.querySelectorAll('section, .stats, .download'));
secs.forEach(function (s) { s.classList.add('reveal'); });
if ('IntersectionObserver' in window) {
  var io = new IntersectionObserver(function (es) {
    es.forEach(function (e) {
      if (e.isIntersecting) {
        var i = secs.indexOf(e.target) % 3;
        e.target.style.transitionDelay = (i * 90) + 'ms';
        e.target.classList.add('show');
        io.unobserve(e.target);
      }
    });
  }, { threshold: 0.08 });
  secs.forEach(function (s) { io.observe(s); });
} else { secs.forEach(function (s) { s.classList.add('show'); }); }

/* ---------- nav scroll state · burger · back-to-top ---------- */
var nav = document.getElementById('nav');
var links = document.getElementById('links');
var burger = document.getElementById('burger');
var topbtn = document.getElementById('topbtn');
function onScroll() {
  var y = window.pageYOffset || document.documentElement.scrollTop;
  if (nav) { nav.classList.toggle('scrolled', y > 16); }
  if (topbtn) { topbtn.classList.toggle('on', y > 560); }
}
window.addEventListener('scroll', onScroll, { passive: true });
onScroll();
if (burger && links) {
  burger.addEventListener('click', function () { links.classList.toggle('open'); });
  Array.prototype.slice.call(links.querySelectorAll('a')).forEach(function (a) {
    a.addEventListener('click', function () { links.classList.remove('open'); });
  });
}
if (topbtn) {
  topbtn.addEventListener('click', function () {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  });
}
})();
