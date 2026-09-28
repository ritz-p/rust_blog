document.addEventListener('DOMContentLoaded', () => {
  for (const code of document.querySelectorAll('.content pre > code')) {
    const toolbar = document.createElement('div');
    toolbar.className = 'code-toolbar';
    const label = document.createElement('span');
    label.textContent = code.dataset.language || 'Text';
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'button is-small';
    button.textContent = 'コードをコピー';
    const status = document.createElement('span');
    status.setAttribute('role', 'status');
    button.addEventListener('click', async () => {
      try {
        await window.navigator.clipboard.writeText(code.textContent);
        status.textContent = 'コピーしました';
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
