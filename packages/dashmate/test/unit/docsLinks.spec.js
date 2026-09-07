import { DOCS_LINKS } from '../../src/docsLinks.js';

describe('DOCS_LINKS', () => {
  // dashmate has already shipped links that answered 404. A command's whole
  // value is that what it tells an operator is true, so a dead link costs more
  // than the guidance it was meant to carry.
  it('should be well-formed documentation links', () => {
    Object.entries(DOCS_LINKS).forEach(([name, url]) => {
      expect(url, name).to.match(/^https:\/\/docs\.dash\.org\/\S+$/);
      expect(url, `${name} has no trailing space`).to.equal(url.trim());
    });
  });

  it('should not repeat a target under two names', () => {
    const urls = Object.values(DOCS_LINKS);

    expect(new Set(urls).size, 'each link appears once').to.equal(urls.length);
  });
});
