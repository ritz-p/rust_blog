document.addEventListener('DOMContentLoaded', () => {
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
        window.location.assign(href);
      }
    });
  }
});
