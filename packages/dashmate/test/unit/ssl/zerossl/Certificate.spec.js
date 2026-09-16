import Certificate from '../../../../src/ssl/zerossl/Certificate.js';

describe('Certificate', () => {
  it('should handle a pending certificate without an expiration date', () => {
    const certificate = new Certificate({
      status: 'pending_validation',
      created: '2026-08-18 09:04:19',
      expires: null,
    });

    expect(certificate.expires).to.be.null();
    expect(certificate.isExpiredInDays(30)).to.be.false();
  });
});
