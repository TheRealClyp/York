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
var dl = {
  windows: '/releases/download/v0.1.0/york-setup-x64.exe',
  macos: '/releases/download/v0.1.0/york-aarch64-apple-darwin.tar.gz',
  linux: '/releases/download/v0.1.0/york-x86_64-linux.tar.gz'
};
var b = document.getElementById('dlbtn');
if (b) {
  var o = os();
  var href = dl[o] || dl.linux;
  var name = o.charAt(0).toUpperCase() + o.slice(1);
  b.href = href;
  b.textContent = 'Download for ' + name;
  var a = href.split('/').pop();
  var d = document.getElementById('dlhint');
  if (d) { d.textContent = 'Auto-detected: ' + name + ' · ' + a + ' — SHA-256 checksums below.'; }
}
})();
