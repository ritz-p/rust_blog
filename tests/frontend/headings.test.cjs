const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { JSDOM } = require('jsdom');

test('heading link copies the current URL and exposes a fallback on failure', async () => {
  const dom = new JSDOM('<div class="content"><h2 id="heading-日本語">日本語</h2></div>', { url: 'https://example.com/posts/test/', runScripts: 'outside-only' });
  try {
    await new Promise(resolve => dom.window.document.addEventListener('DOMContentLoaded', resolve, { once: true }));
    let copied;
    Object.defineProperty(dom.window.navigator, 'clipboard', { value: { writeText: async value => { copied = value; } } });
    dom.window.eval(readFileSync('core/assets/nav.js', 'utf8'));
    dom.window.document.dispatchEvent(new dom.window.Event('DOMContentLoaded'));
    const button = dom.window.document.querySelector('.heading-copy');
    button.click();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(decodeURIComponent(copied), 'https://example.com/posts/test/#heading-日本語');
    dom.window.navigator.clipboard.writeText = async () => { throw new Error('denied'); };
    button.click();
    await new Promise(resolve => setImmediate(resolve));
    assert.match(dom.window.document.querySelector('[role=status]').textContent, /コピーできません/);
    assert.equal(dom.window.document.querySelector('a').getAttribute('href'), '#heading-日本語');
  } finally { dom.window.close(); }
});

test('legacy fragments highlight and focus the heading on load and hash changes', async () => {
  const dom = new JSDOM(`<style>${readFileSync('core/assets/site.css', 'utf8')}</style>
    <div class="content"><span id="toc-heading-1"></span><h2 id="heading-first">First</h2>
    <span id="toc-heading-2"></span><h3 id="heading-second">Second</h3></div>`, {
    url: 'https://example.com/posts/test/#toc-heading-1', runScripts: 'outside-only',
  });
  try {
    await new Promise(resolve => dom.window.document.addEventListener('DOMContentLoaded', resolve, { once: true }));
    const scrolled = [];
    dom.window.HTMLElement.prototype.scrollIntoView = function (options) {
      assert.equal(options.block, 'start');
      scrolled.push(this.id);
    };
    dom.window.eval(readFileSync('core/assets/nav.js', 'utf8'));
    dom.window.document.dispatchEvent(new dom.window.Event('DOMContentLoaded'));
    const first = dom.window.document.getElementById('heading-first');
    assert.equal(dom.window.document.activeElement, first);
    assert.equal(dom.window.getComputedStyle(first).textDecoration, 'underline');
    dom.window.location.hash = '#toc-heading-2';
    await new Promise(resolve => dom.window.addEventListener('hashchange', resolve, { once: true }));
    const second = dom.window.document.getElementById('heading-second');
    assert.equal(dom.window.document.activeElement, second);
    assert.equal(dom.window.getComputedStyle(second).textDecoration, 'underline');
    assert.notEqual(dom.window.getComputedStyle(first).textDecoration, 'underline');
    assert.deepEqual(scrolled, ['heading-first', 'heading-second']);
  } finally { dom.window.close(); }
});
