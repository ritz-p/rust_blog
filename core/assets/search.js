document.addEventListener('DOMContentLoaded', async () => {
  const results = document.getElementById('index-results');
  if (!results || results.dataset.staticSearch !== 'true') return;
  const params = new URLSearchParams(window.location.search);
  const query = (params.get('q') || '').trim();
  const year = params.get('year') || results.dataset.year;
  const month = params.get('month') || results.dataset.month;
  const validPeriod = /^\d{4}$/.test(year) && Number(month) >= 1 && Number(month) <= 12;
  const periodPath = validPeriod ? `/archive/${year}/${String(Number(month)).padStart(2, '0')}/` : '/';
  if (!query) {
    if (validPeriod && window.location.pathname === '/') window.location.replace(periodPath);
    return;
  }
  const input = document.getElementById('search-query');
  const status = document.getElementById('search-status');
  const form = document.getElementById('article-search');
  input.value = query;
  document.getElementById('clear-search').href = periodPath;
  const per = Math.max(1, Number.parseInt(params.get('per'), 10) || Number(results.dataset.per));
  form.elements.per.value = String(per);
  for (const [name, value] of [['year', year], ['month', month]]) {
    if (!value) continue;
    let field = form.elements[name];
    if (!field) {
      field = document.createElement('input');
      field.type = 'hidden';
      field.name = name;
      form.append(field);
    }
    field.value = value;
  }
  for (const option of document.querySelectorAll('[data-period-filter] option')) {
    option.selected = option.dataset.href === periodPath;
  }
  results.replaceChildren();
  status.textContent = '検索中…';

  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  function pageUrl(page) {
    const url = new URL('/', window.location.origin);
    url.searchParams.set('q', query);
    url.searchParams.set('page', String(page));
    url.searchParams.set('per', String(per));
    if (year) url.searchParams.set('year', year);
    if (month) url.searchParams.set('month', month);
    return url.pathname + url.search;
  }

  function link(text, page, className, disabled = false, current = false) {
    const node = element(disabled || current ? 'span' : 'a', className, text);
    if (disabled) node.setAttribute('aria-disabled', 'true');
    if (current) node.setAttribute('aria-current', 'page');
    if (!disabled && !current) node.href = pageUrl(page);
    return node;
  }

  try {
    const response = await fetch('/search-index.json', { cache: 'no-cache' });
    if (!response.ok) throw new Error('search index unavailable');
    const articles = await response.json();
    const terms = query.toLowerCase().split(/\s+/u).filter(Boolean);
    const matches = articles.filter(article =>
      terms.every(term => article.text.includes(term)) &&
      (!(year || month) || (validPeriod && article.year === Number(year) && article.month === Number(month)))
    );
    const totalPages = Math.max(1, Math.ceil(matches.length / per));
    const page = Math.min(totalPages, Math.max(1, Number.parseInt(params.get('page'), 10) || 1));
    status.textContent = `「${query}」の検索結果: ${matches.length}件`;
    if (!matches.length) {
      results.append(element('p', 'has-text-grey', '一致する記事がありません。'));
      return;
    }
    const list = element('div', 'article-list');
    for (const article of matches.slice((page - 1) * per, page * per)) {
      const card = element('article', 'article-card');
      if (article.icatch_path) {
        const imageUrl = new URL(article.icatch_path, window.location.origin);
        if (['http:', 'https:'].includes(imageUrl.protocol)) {
          const imageLink = element('a', 'article-card-image');
          imageLink.href = article.url;
          imageLink.tabIndex = -1;
          imageLink.setAttribute('aria-hidden', 'true');
          const image = element('img');
          Object.assign(image, { src: imageUrl.href, alt: '', loading: 'lazy', decoding: 'async', width: 240, height: 160 });
          imageLink.append(image);
          card.append(imageLink);
        }
      }
      const body = element('div', 'article-card-body');
      const date = element('p', 'article-card-date');
      date.append(element('span', 'date-item', `公開日 ${article.created_at}`));
      date.append(element('span', 'date-item date-updated', ` / 更新日 ${article.updated_at}`));
      const title = element('h2', 'article-card-title');
      const titleLink = element('a', '', article.title);
      titleLink.href = article.url;
      title.append(titleLink);
      body.append(date, title);
      if (article.excerpt) body.append(element('p', 'article-card-excerpt', article.excerpt));
      card.append(body);
      list.append(card);
    }
    results.append(list);
    const nav = element('nav', 'pagination is-centered');
    nav.setAttribute('aria-label', 'pagination');
    nav.append(link('Prev', page - 1, 'pagination-previous', page === 1));
    nav.append(link('Next', page + 1, 'pagination-next', page === totalPages));
    const pages = element('ul', 'pagination-list');
    function addPage(node) {
      const item = element('li');
      item.append(node);
      pages.append(item);
    }
    addPage(link('Latest', 1, 'pagination-link', page === 1));
    for (let number = Math.max(1, page - 3); number <= Math.min(totalPages, page + 3); number++) {
      addPage(link(String(number), number, `pagination-link${number === page ? ' is-current' : ''}`, false, number === page));
    }
    addPage(link('Oldest', totalPages, 'pagination-link', page === totalPages));
    nav.append(pages);
    results.append(nav, element('p', 'has-text-centered', `Page ${page} / ${totalPages}`));
  } catch (error) {
    status.textContent = '検索データを読み込めませんでした。ページを再読み込みしてください。';
  }
});
