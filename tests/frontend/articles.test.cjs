const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { JSDOM } = require('jsdom');

const base = process.env.STATIC_SITE_URL;
test('confirmation articles preserve code and display conditions through nginx', { skip: !base && 'Set STATIC_SITE_URL to check the exported fixtures' }, async () => {
  for (const number of [31, 33, 34, 35, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49]) {
    const response = await fetch(new URL(`/posts/${number}/`, base));
    if (number === 37) { assert.equal(response.status, 404); continue; }
    assert.equal(response.status, 200, `article ${number}`);
    const dom = new JSDOM(await response.text());
    try {
      const content = dom.window.document.querySelector('.content.is-medium');
      assert.ok(content, `article ${number}`);
      const source = readFileSync(`content/articles/${number}.md`, 'utf8').replaceAll('\r\n', '\n');
      const blocks = [...source.matchAll(/^```[^\n]*\n([\s\S]*?)^```/gm)];
      const rendered = [...content.querySelectorAll('pre code')];
      assert.equal(rendered.length, blocks.length, `code block count ${number}`);
      for (const [index, block] of blocks.entries()) {
        assert.equal(rendered[index].textContent, block[1], `source retained: ${number}/${index}`);
        if (![31, 44].includes(number)) assert.ok(rendered[index].querySelector('[class*="syntax-"]'), `highlight ${number}/${index}`);
      }
      if ([31, 46].includes(number)) {
        for (const link of content.querySelectorAll('nav.table-of-contents a')) {
          assert.ok(content.querySelector(`[id="${link.hash.slice(1)}"]`), `TOC target ${number}`);
        }
      }
      if (number === 38) assert.equal(content.querySelector('nav.table-of-contents'), null);
      if (number === 47) assert.equal(content.textContent.trim(), '');
      if (number === 48) {
        const image = content.querySelector('img');
        assert.ok(image.alt);
        assert.equal((await fetch(new URL(image.getAttribute('src'), base))).status, 200);
      }
    } finally { dom.window.close(); }
  }
});

test.todo('PowerShell highlighting: enable strict syntax assertion for article 44 after #194');
test.todo('Footnote forward/back links: assert all fragments after #193');
