const adSlot = document.querySelector('ins.adsbygoogle[data-ad-client]');
if (adSlot) {
  const script = document.createElement('script');
  script.async = true;
  script.crossOrigin = 'anonymous';
  script.src = 'https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=' +
    encodeURIComponent(adSlot.dataset.adClient);
  document.head.append(script);
  (window.adsbygoogle = window.adsbygoogle || []).push({});
}
