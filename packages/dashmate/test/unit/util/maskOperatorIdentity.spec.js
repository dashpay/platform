import maskOperatorIdentity from '../../../src/util/maskOperatorIdentity.js';

describe('maskOperatorIdentity', () => {
  // What actually discloses who is running dashmate is the home directory in
  // an absolute path, not the word itself.
  it('should mask the home directory while leaving the path usable', () => {
    const masked = maskOperatorIdentity(
      '/home/alice/.dashmate/base/platform/gateway/ssl/bundle.crt',
      { username: 'alice', homePath: '/home/alice' },
    );

    expect(masked).to.not.contain('alice');
    expect(masked).to.equal('~/.dashmate/base/platform/gateway/ssl/bundle.crt');
  });

  it('should mask the name as a whole word elsewhere', () => {
    expect(maskOperatorIdentity('user alice ran it', { username: 'alice', homePath: '/home/alice' }))
      .to.not.contain('alice');
  });

  // Substring masking turned "malice" into "m********" and any word containing
  // the name into nonsense.
  it('should not mask a word that merely contains the name', () => {
    expect(maskOperatorIdentity('malice and alicia', { username: 'alice', homePath: '/home/alice' }))
      .to.equal('malice and alicia');
  });

  // On a deb-installed node the service account is called dashmate, so the
  // username and the product name are the same string. Masking it blanks the
  // subject out of every sentence dashmate writes about itself - "******** could
  // not find the certificate bundle" - and mangles the directories dashmate
  // itself creates, while hiding nothing the home path has not already hidden.
  describe('when the operator is named after the product', () => {
    const identity = { username: 'dashmate', homePath: '/home/dashmate' };

    it('should still remove the home directory', () => {
      const masked = maskOperatorIdentity(
        '/home/dashmate/.dashmate-ssltest/ssltest/platform/gateway/ssl/bundle.crt',
        identity,
      );

      expect(masked).to.not.contain('/home/dashmate');
      expect(masked).to.equal('~/.dashmate-ssltest/ssltest/platform/gateway/ssl/bundle.crt');
    });

    it('should leave the sentence readable', () => {
      expect(maskOperatorIdentity('dashmate could not find the certificate bundle', identity))
        .to.equal('dashmate could not find the certificate bundle');
    });
  });

  it('should leave text alone when no identity is known', () => {
    expect(maskOperatorIdentity('/home/alice/x', { username: null, homePath: null }))
      .to.equal('/home/alice/x');
  });

  it('should pass through anything that is not a string', () => {
    expect(maskOperatorIdentity(42, { username: 'alice', homePath: '/home/alice' })).to.equal(42);
  });
});
