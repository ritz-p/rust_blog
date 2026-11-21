const loader = document.currentScript;
const script = document.createElement('script');
script.async = true;
script.crossOrigin = 'anonymous';
script.src = 'https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=' +
  encodeURIComponent(loader.dataset.adClient);
document.head.append(script);

if (loader.hasAttribute('data-sidebar')) {
  (window.adsbygoogle = window.adsbygoogle || []).push({});
}
