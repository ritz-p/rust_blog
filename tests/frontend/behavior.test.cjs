const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const { JSDOM } = require('jsdom');

const fixture = `<!doctype html><html><body>
  <button class="navbar-burger" data-target="menu"></button><div id="menu"></div>
  <select data-period-filter><option data-href="/">All</option>
    <option data-href="/archive/2026/09/">September</option></select>
  <form id="article-search"><input id="search-query"><input name="per"></form>
  <a id="clear-search"></a><p id="search-status"></p>
  <div id="index-results" data-static-search="true" data-per="2"></div>
</body></html>`;

async function browser(t, script, query = '', fetch = undefined) {
  const dom = new JSDOM(fixture, { url: `https://blog.example/${query}` });
  t.after(() => dom.window.close());
  await new Promise(resolve => dom.window.document.addEventListener('DOMContentLoaded', resolve, { once: true }));
  let handler;
  const document = dom.window.document;
  const addEventListener = document.addEventListener.bind(document);
  document.addEventListener = (event, callback, options) => {
    if (event === 'DOMContentLoaded') handler = callback;
    else addEventListener(event, callback, options);
  };
  const navigations = [];
  const location = {
    origin: dom.window.location.origin,
    pathname: '/', search: dom.window.location.search,
    assign: href => navigations.push(href), replace: href => navigations.push(href),
  };
  const source = readFileSync(path.join(__dirname, '../../core/assets', script), 'utf8');
  vm.runInNewContext(source, { document, window: { location }, URL, URLSearchParams, fetch });
  await handler();
  return { document, navigations };
}

test('menu toggles and month navigation preserves the search query', async t => {
  const { document, navigations } = await browser(t, 'nav.js', '?q=%20Rust%20日本語%20');
  const button = document.querySelector('.navbar-burger');
  button.click();
  assert.ok(button.classList.contains('is-active'));
  assert.ok(document.getElementById('menu').classList.contains('is-active'));
  button.click();
  assert.ok(!document.getElementById('menu').classList.contains('is-active'));
  const select = document.querySelector('select');
  select.selectedIndex = 1;
  select.dispatchEvent(new document.defaultView.Event('change'));
  const url = new URL(navigations[0]);
  assert.equal(url.pathname, '/archive/2026/09/');
  assert.equal(url.searchParams.get('q'), 'Rust 日本語');
});

const articles = Array.from({ length: 5 }, (_, index) => ({
  title: `<img src=x onerror=alert(1)> Rust ${index}`,
  text: `rust 日本語 ${index}`, url: `/posts/${index}/`, excerpt: '<b>plain text</b>',
  year: 2026, month: 9, created_at: '2026-09-01', updated_at: '2026-09-01',
}));

test('search combines terms, paginates, preserves period and renders text safely', async t => {
  const { document } = await browser(t, 'search.js', '?q=RUST+日本語&page=2&year=2026&month=9', async (url, options) => {
    assert.equal(url, '/search-index.json');
    assert.equal(options.cache, 'no-cache');
    return { ok: true, json: async () => [...articles, { ...articles[0], month: 8 }, { ...articles[0], text: 'rust only' }] };
  });
  assert.equal(document.querySelectorAll('.article-card').length, 2);
  assert.match(document.getElementById('search-status').textContent, /5件/);
  assert.match(document.querySelector('.article-card-title').textContent, /Rust 2$/);
  assert.equal(document.querySelectorAll('.article-card img, .article-card b').length, 0);
  const next = new URL(document.querySelector('.pagination-next').href);
  assert.equal(next.searchParams.get('q'), 'RUST 日本語');
  assert.equal(next.searchParams.get('page'), '3');
  assert.equal(next.searchParams.get('month'), '9');
  assert.equal(document.getElementById('clear-search').pathname, '/archive/2026/09/');
  assert.equal(document.querySelector('nav.pagination').nextElementSibling.textContent, 'Page 2 / 3');
});

test('empty results and fetch failures produce visible status messages', async t => {
  const empty = await browser(t, 'search.js', '?q=missing', async () => ({ ok: true, json: async () => articles }));
  assert.match(empty.document.getElementById('index-results').textContent, /一致する記事がありません/);
  const failed = await browser(t, 'search.js', '?q=rust', async () => ({ ok: false }));
  assert.match(failed.document.getElementById('search-status').textContent, /読み込めません/);
});

test('an empty query does not fetch the search index', async t => {
  await browser(t, 'search.js', '?q=%20', () => assert.fail('unexpected fetch'));
});

test('static preview serves the referenced scripts and searchable documents', { skip: !process.env.STATIC_SITE_URL }, async () => {
  const base = process.env.STATIC_SITE_URL;
  const response = await fetch(base);
  assert.ok(response.ok);
  const dom = new JSDOM(await response.text(), { url: base });
  try {
    const scripts = [...dom.window.document.querySelectorAll('script[src]')];
    for (const name of ['nav.js', 'search.js']) {
      assert.ok(scripts.some(script => new URL(script.src).pathname === `/js/${name}`));
    }
    for (const script of scripts) {
      const asset = await fetch(script.src);
      assert.ok(asset.ok, script.src);
      assert.match(asset.headers.get('content-type'), /javascript/);
    }
    const response = await fetch(new URL('/search-index.json', base));
    assert.ok(response.ok);
    const records = await response.json();
    assert.ok(records.length > 0);
    const article = await fetch(new URL(records[0].url, base));
    assert.ok(article.ok);
    assert.ok(dom.window.document.querySelector('[data-static-search="true"]'));
  } finally {
    dom.window.close();
  }
});
