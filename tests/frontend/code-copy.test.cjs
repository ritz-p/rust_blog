const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { JSDOM } = require('jsdom');

test('copy controls preserve source and select the correct block when clipboard access fails', async () => {
  const dom = new JSDOM('<div class="content"><pre><code data-language="rust"><span>\t世界 &lt;x&gt;</span>\n</code></pre><pre><code>second\n</code></pre></div>', { runScripts: 'outside-only' });
  try {
    await new Promise(resolve => dom.window.document.addEventListener('DOMContentLoaded', resolve, { once: true }));
    let copied;
    Object.defineProperty(dom.window.navigator, 'clipboard', { value: { writeText: async value => { copied = value; } } });
    dom.window.eval(readFileSync('core/assets/nav.js', 'utf8'));
    dom.window.document.dispatchEvent(new dom.window.Event('DOMContentLoaded'));
    const toolbars = dom.window.document.querySelectorAll('.code-toolbar');
    assert.equal(toolbars.length, 2);
    assert.equal(toolbars[0].firstElementChild.textContent, 'rust');
    assert.equal(toolbars[1].firstElementChild.textContent, 'Text');
    toolbars[0].querySelector('button').click();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(copied, '\t世界 <x>\n');
    assert.match(toolbars[0].textContent, /コピーしました/);
    dom.window.navigator.clipboard.writeText = async () => { throw new Error('denied'); };
    toolbars[1].querySelector('button').click();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(dom.window.getSelection().toString(), 'second\n');
    assert.match(toolbars[1].textContent, /手動でコピー/);
  } finally { dom.window.close(); }
});
