/* Render the existing diagram source independently of the MkDocs theme. */
(async () => {
  if (document.readyState === 'loading') {
    await new Promise(resolve => document.addEventListener('DOMContentLoaded', resolve, { once: true }));
  }
  const nodes = Array.from(document.querySelectorAll('pre.mermaid'));
  if (!nodes.length) return;
  const sources = nodes.map(node => node.textContent);
  try {
    const { default: mermaid } = await import('https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.esm.min.mjs');
    await document.fonts.ready;
    let pending = Promise.resolve();
    const render = () => {
      pending = pending.then(async () => {
        const dark = document.documentElement.dataset.tutorialPalette === 'dark';
        mermaid.initialize({ startOnLoad: false, securityLevel: 'strict',
          theme: dark ? 'dark' : 'default', fontFamily: 'Arial, sans-serif',
          htmlLabels: false });
        nodes.forEach((node, index) => {
          node.textContent = sources[index];
          node.removeAttribute('data-processed');
          node.style.backgroundColor = dark ? '#1f2020' : '#ffffff';
          node.style.overflowX = 'auto';
        });
        await mermaid.run({ nodes });
      }).catch(error => { console.error('Diagram rendering failed', error); });
    };
    render();
    new MutationObserver(render).observe(document.documentElement,
      { attributes: true, attributeFilter: ['data-tutorial-palette'] });
  } catch (error) {
    console.error('Diagram renderer unavailable', error);
  }
})();
