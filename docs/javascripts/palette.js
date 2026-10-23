/* Follow the device preference until the reader chooses a palette. */
(() => {
  const icon = body => '<svg viewBox="0 0 16 16" width="1.1em" height="1.1em" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + body + '</svg>';
  const SUN = icon('<circle cx="8" cy="8" r="3"/><path d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1"/>');
  const MOON = icon('<path d="M13.5 9.5A5.5 5.5 0 0 1 6.5 2.5a5.5 5.5 0 1 0 7 7z"/>');
  const system = window.matchMedia('(prefers-color-scheme: dark)');
  let choice;
  try { choice = localStorage.getItem('tutorial-stepper-palette'); } catch (_) { /* Storage is optional. */ }
  const isDark = () => choice === 'dark' || (choice !== 'light' && system.matches);
  const apply = () => {
    const dark = isDark();
    document.documentElement.dataset.tutorialPalette = dark ? 'dark' : 'light';
    document.getElementById('tutorial-dark-palette').media = dark ? 'all' : 'not all';
    document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
    const button = document.getElementById('tutorial-palette-toggle');
    if (button) {
      button.innerHTML = dark ? SUN : MOON;
      button.title = dark ? 'Switch to light mode' : 'Switch to dark mode';
      button.setAttribute('aria-label', dark ? 'Switch to light mode' : 'Switch to dark mode');
      button.hidden = false;
    }
  };
  apply();
  system.addEventListener('change', apply);
  document.addEventListener('DOMContentLoaded', () => {
    apply();
    document.getElementById('tutorial-palette-toggle').addEventListener('click', () => {
      choice = isDark() ? 'light' : 'dark';
      try { localStorage.setItem('tutorial-stepper-palette', choice); } catch (_) { /* Storage is optional. */ }
      apply();
    });
    const navigation = document.getElementById('tutorial-navigation');
    if (navigation) {
      const wide = window.matchMedia('(min-width: 70em)');
      navigation.open = wide.matches;
      wide.addEventListener('change', () => { navigation.open = wide.matches; });
    }
  });
})();
