const fs = require('node:fs/promises');
const path = require('node:path');
const { JSDOM } = require('jsdom');

async function checkLinks(root, { baseUrl, exceptions = [] } = {}) {
  root = path.resolve(root);
  const origin = new URL(baseUrl || 'https://local.invalid').origin;
  const files = new Set();
  async function walk(dir) {
    for (const item of await fs.readdir(dir, { withFileTypes: true })) {
      const full = path.join(dir, item.name);
      if (item.isDirectory()) await walk(full);
      else if (item.isFile()) files.add(path.relative(root, full).split(path.sep).join('/'));
    }
  }
  await walk(root);
  const pages = new Map();
  const errors = [];
  const ignored = [];
  const used = new Set();
  function report(file, href, reason) {
    const index = exceptions.findIndex(item => item.file === file && item.href === href && item.reason === reason && item.note);
    const failure = { file, href, reason };
    if (index >= 0) { used.add(index); ignored.push(failure); }
    else errors.push(failure);
  }
  function pageUrl(file) {
    return '/' + file.split('/').map(encodeURIComponent).join('/').replace(/index\.html$/, '');
  }
  for (const file of files) {
    if (!file.endsWith('.html')) continue;
    let html;
    if (baseUrl) {
      const response = await fetch(new URL(pageUrl(file), origin), { signal: AbortSignal.timeout(15000) });
      if (!response.ok) { report(file, pageUrl(file), `HTTP ${response.status}`); continue; }
      html = await response.text();
    } else html = await fs.readFile(path.join(root, file), 'utf8');
    const dom = new JSDOM(html);
    const document = dom.window.document;
    pages.set(file, {
      ids: new Set([...document.querySelectorAll('[id], a[name]')].flatMap(node => [node.id, node.getAttribute('name')]).filter(Boolean)),
      links: [...document.querySelectorAll('a[href], img[src], link[href], script[src]')].map(node => ({
        href: node.getAttribute(node.hasAttribute('src') ? 'src' : 'href'),
        fragment: node.tagName === 'A',
      })),
    });
    dom.window.close();
  }
  for (const [file, page] of pages) {
    for (const { href, fragment } of page.links) {
      let url, relative, hash;
      try {
        url = new URL(href, new URL(pageUrl(file), origin));
        if (!['http:', 'https:'].includes(url.protocol) || url.origin !== origin) continue;
        relative = decodeURIComponent(url.pathname).replace(/^\//, '');
        hash = decodeURIComponent(url.hash.slice(1));
      } catch { report(file, href, 'invalid URL encoding'); continue; }
      if (relative.includes('\\') || relative.split('/').includes('..')) { report(file, href, 'unsafe path'); continue; }
      const target = files.has(relative) ? relative : `${relative.replace(/\/$/, '')}${relative.replace(/\/$/, '') ? '/' : ''}index.html`;
      if (!files.has(target)) { report(file, href, 'missing target'); continue; }
      if (fragment && hash && !hash.startsWith(':~:text=') && (!pages.has(target) || !pages.get(target).ids.has(hash))) {
        if (target.endsWith('.html')) report(file, href, 'missing fragment');
      }
    }
  }
  exceptions.forEach((item, index) => {
    if (!used.has(index)) errors.push({ file: item.file, href: item.href, reason: 'unused exception; remove it' });
  });
  return { errors, ignored, pages: pages.size };
}

if (require.main === module) {
  (async () => {
    const exceptions = JSON.parse(await fs.readFile('tests/frontend/link-exceptions.json', 'utf8'));
    const result = await checkLinks(process.argv[2] || 'dist', { baseUrl: process.env.STATIC_SITE_URL, exceptions });
    for (const error of result.errors) console.error(`${error.file}: ${error.href}: ${error.reason}`);
    console.log(`Links: ${result.pages} pages, ${result.errors.length} errors, ${result.ignored.length} expected failures`);
    process.exitCode = result.errors.length ? 1 : 0;
  })().catch(error => { console.error(error); process.exitCode = 1; });
}

module.exports = { checkLinks };
