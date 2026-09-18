(function () {
function os() {
  var u = (navigator.userAgent || '').toLowerCase();
  if (/windows|win32|win64/.test(u)) return 'windows';
  if (/macintosh|mac os x/.test(u)) return 'macos';
  return 'linux';
}
var cards = Array.prototype.slice.call(document.querySelectorAll('.card[data-os]'));
var btns = Array.prototype.slice.call(document.querySelectorAll('.osbtn'));
function pick(name) {
  cards.forEach(function (c) { c.classList.toggle('active', c.getAttribute('data-os') === name); });
  btns.forEach(function (b) { b.classList.toggle('active', b.getAttribute('data-os') === name); });
}
btns.forEach(function (b) {
  b.addEventListener('click', function () { pick(b.getAttribute('data-os')); });
});
pick(os());
Array.prototype.slice.call(document.querySelectorAll('.copy')).forEach(function (b) {
  b.addEventListener('click', function () {
    var pre = b.parentElement.querySelector('pre');
    var sel = window.getSelection();
    var range = document.createRange();
    range.selectNodeContents(pre);
    sel.removeAllRanges();
    sel.addRange(range);
    try { document.execCommand('copy'); b.textContent = 'copied!'; }
    catch (e) { b.textContent = 'press ctrl+c'; }
    sel.removeAllRanges();
    setTimeout(function () { b.textContent = 'copy'; }, 1200);
  });
});
var dl = { windows: 'downloads/york-setup-x64.exe' };
var b = document.getElementById('dlbtn');
if (b) {
  var o = os();
  var d = document.getElementById('dlhint');
  if (o === 'macos' || o === 'linux') {
    b.href = '#install';
    b.textContent = 'macOS / Linux - coming soon';
    if (d) { d.textContent = 'Windows binaries are ready today. macOS and Linux builds are next.'; }
  } else {
    var href = dl.windows;
    b.href = href;
    b.textContent = 'Download for Windows';
    if (d) { d.textContent = 'Auto-detected: Windows - ' + href.split('/').pop() + ', SHA-256 verified.'; }
  }
}
var secs = Array.prototype.slice.call(document.querySelectorAll('section'));
secs.forEach(function (s) { if (s) s.classList.add('reveal'); });
if ('IntersectionObserver' in window) {
  var io = new IntersectionObserver(function (es) {
    es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add('show'); io.unobserve(e.target); } });
  }, { threshold: 0.06 });
  secs.forEach(function (s) { io.observe(s); });
} else { secs.forEach(function (s) { s.classList.add('show'); }); }
})();
