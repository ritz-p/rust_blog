const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { JSDOM } = require('jsdom');

test('PowerShell variables have a visible syntax color distinct from ordinary code', () => {
  const dom = new JSDOM('<div class="content"><pre><code><span class="syntax-variable syntax-other syntax-powershell">$names</span><span id="plain"> ordinary text</span></code></pre></div>');
  try {
    const document = dom.window.document;
    const style = document.createElement('style');
    style.textContent = readFileSync('core/assets/site.css', 'utf8');
    document.head.append(style);
    const variable = dom.window.getComputedStyle(document.querySelector('.syntax-variable')).color;
    const plain = dom.window.getComputedStyle(document.querySelector('#plain')).color;
    assert.equal(variable, 'rgb(5, 80, 174)');
    assert.notEqual(variable, plain);
  } finally { dom.window.close(); }
});
