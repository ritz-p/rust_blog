document.addEventListener('DOMContentLoaded', () => {
  if (window.matchMedia?.('(max-width: 768px)').matches) {
    for (const toc of document.querySelectorAll('.table-of-contents details')) {
      if (toc.querySelectorAll('li').length > 6) toc.open = false;
    }
  }
  for (const image of document.querySelectorAll('.article-page .content img[src]')) {
    if (image.closest('a') || !/^https?:/i.test(image.src)) continue;
    const link = document.createElement('a');
    link.href = image.src;
    link.setAttribute('aria-label', (image.alt || '画像') + 'を原寸で開く');
    image.before(link);
    link.append(image);
  }
  for (const code of document.querySelectorAll('.content pre > code')) {
    const toolbar = document.createElement('div');
    toolbar.className = 'code-toolbar';
    const label = document.createElement('span');
    label.textContent = code.dataset.language || 'Text';
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'button is-small';
    button.setAttribute('aria-label', 'コードをコピー');
    button.innerHTML = '<svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/></svg>';
    const status = document.createElement('span');
    status.setAttribute('role', 'status');
    button.addEventListener('click', async () => {
      try {
        await window.navigator.clipboard.writeText(code.textContent);
        status.textContent = '';
      } catch {
        const range = document.createRange();
        range.selectNodeContents(code);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        status.textContent = 'コピーできません。選択したコードを手動でコピーしてください。';
      }
    });
    toolbar.append(label, button, status);
    code.parentElement.before(toolbar);
  }
  const focusLegacyHeading = () => {
    let id;
    try {
      id = decodeURIComponent(window.location.hash.slice(1));
    } catch {
      return;
    }
    if (!/^toc-heading-\d+$/.test(id)) return;
    const anchor = document.getElementById(id);
    const heading = anchor?.nextElementSibling;
    if (anchor?.tagName !== 'SPAN' || !/^H[1-6]$/.test(heading?.tagName || '')) return;
    heading.tabIndex = -1;
    heading.focus({ preventScroll: true });
    heading.scrollIntoView({ block: 'start' });
  };
  window.addEventListener('hashchange', focusLegacyHeading);
  focusLegacyHeading();
  for (const heading of document.querySelectorAll('.content :is(h1,h2,h3,h4,h5,h6)[id^="heading-"]')) {
    const link = document.createElement('a');
    link.href = `#${heading.id}`;
    link.textContent = ' #';
    link.setAttribute('aria-label', '見出しへのリンク');
    heading.append(link);
  }
  const burgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);
  burgers.forEach((el) => {
    el.addEventListener('click', () => {
      const target = el.dataset.target;
      const targetElement = document.getElementById(target);
      if (targetElement) {
        const expanded = targetElement.classList.toggle('is-active');
        el.classList.toggle('is-active', expanded);
        el.setAttribute('aria-expanded', String(expanded));
      }
    });
  });

  const periodFilter = document.querySelector('[data-period-filter]');
  if (periodFilter) {
    periodFilter.addEventListener('change', (event) => {
      const select = event.currentTarget;
      const option = select.options[select.selectedIndex];
      const query = (new URLSearchParams(window.location.search).get('q') || '').trim();
      const href = option ? ((query && option.dataset.searchHref) || option.dataset.href) : '';
      if (href) {
        const url = new URL(href, window.location.origin);
        if (query) url.searchParams.set('q', query);
        window.location.assign(url.href);
      }
    });
  }
});
