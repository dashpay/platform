// Mermaid initialization for mdBook (without preprocessor)
// Converts ```mermaid code blocks into rendered diagrams.
(() => {
    const darkThemes = ['ayu', 'navy', 'coal'];
    const lightThemes = ['light', 'rust'];

    const classList = document.getElementsByTagName('html')[0].classList;

    let lastThemeWasLight = true;
    for (const cssClass of classList) {
        if (darkThemes.includes(cssClass)) {
            lastThemeWasLight = false;
            break;
        }
    }

    const theme = lastThemeWasLight ? 'default' : 'dark';

    // Convert code blocks with language-mermaid into mermaid divs
    document.querySelectorAll('pre code.language-mermaid').forEach((codeBlock) => {
        const pre = codeBlock.parentElement;
        const div = document.createElement('div');
        div.className = 'mermaid';
        div.textContent = codeBlock.textContent;
        pre.parentElement.replaceChild(div, pre);
    });

    mermaid.initialize({ startOnLoad: true, theme });

    // Re-render on theme switch
    for (const darkTheme of darkThemes) {
        const el = document.getElementById(darkTheme);
        if (el) el.addEventListener('click', () => {
            if (lastThemeWasLight) window.location.reload();
        });
    }
    for (const lightTheme of lightThemes) {
        const el = document.getElementById(lightTheme);
        if (el) el.addEventListener('click', () => {
            if (!lastThemeWasLight) window.location.reload();
        });
    }
})();
