document.addEventListener('DOMContentLoaded', () => {
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
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = 'リンクをコピー';
    button.className = 'button is-small heading-copy';
    const status = document.createElement('span');
    status.setAttribute('role', 'status');
    button.addEventListener('click', async () => {
      const url = new URL(window.location.href);
      url.hash = heading.id;
      try {
        await window.navigator.clipboard.writeText(url.href);
        status.textContent = 'コピーしました';
      } catch {
        status.textContent = 'コピーできません。# のリンクから URL を取得してください。';
      }
    });
    heading.append(link, button, status);
  }
  const burgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);
  burgers.forEach((el) => {
    el.addEventListener('click', () => {
      const target = el.dataset.target;
      const targetElement = document.getElementById(target);
      el.classList.toggle('is-active');
      if (targetElement) {
        targetElement.classList.toggle('is-active');
      }
    });
  });

  const periodFilter = document.querySelector('[data-period-filter]');
  if (periodFilter) {
    periodFilter.addEventListener('change', (event) => {
      const select = event.currentTarget;
      const option = select.options[select.selectedIndex];
      const href = option ? option.dataset.href : '';
      if (href) {
        const url = new URL(href, window.location.origin);
        const query = new URLSearchParams(window.location.search).get('q');
        if (query && query.trim()) url.searchParams.set('q', query.trim());
        window.location.assign(url.href);
      }
    });
  }
});
