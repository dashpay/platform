const { expect } = require('chai');
const sanitizeUrl = require('../../../lib/util/sanitizeUrl');

describe('sanitizeUrl', () => {
  it('should sanitize an url', () => {
    const sanitized = sanitizeUrl('https://www.dash.org?something=true');
    expect(sanitized).to.equal('https://www.dash.org');
  });
  it('should handle non RFC path', () => {
    const sanitized = sanitizeUrl('/foo;jsessionid=123456');
    expect(sanitized).to.equal('/foo');
  });
});
