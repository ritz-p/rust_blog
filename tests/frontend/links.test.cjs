const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { checkLinks } = require('./check-links.cjs');

test('links resolve relative paths, encoded slugs, fragments and both article URL forms', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'blog-links-'));
  try {
    await fs.mkdir(path.join(root, 'posts/記事'), { recursive: true });
    await fs.writeFile(path.join(root, 'index.html'), '<a href="/posts/%E8%A8%98%E4%BA%8B?q=test#見出し">ok</a><a href="/posts/%E8%A8%98%E4%BA%8B/">ok</a>');
    await fs.writeFile(path.join(root, 'posts/記事/index.html'), '<h2 id="見出し">Heading</h2><a href="../../">home</a><a href="https://example.com/missing">external</a><img src="../../image.png">');
    await fs.writeFile(path.join(root, 'image.png'), 'image');
    assert.deepEqual((await checkLinks(root)).errors, []);
    await fs.appendFile(path.join(root, 'index.html'), '<a href="/missing">bad</a><a href="/posts/記事/#missing">bad</a><img src="/absent.png"><a href="/%ZZ">bad</a>');
    const result = await checkLinks(root);
    assert.equal(result.errors.length, 4);
    assert.ok(result.errors.every(error => error.file === 'index.html'));
    const exceptions = result.errors.map(error => ({ ...error, note: 'test' }));
    assert.equal((await checkLinks(root, { exceptions })).errors.length, 0);
    await fs.writeFile(path.join(root, 'index.html'), 'fixed');
    assert.equal((await checkLinks(root, { exceptions })).errors.length, 4);
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});
