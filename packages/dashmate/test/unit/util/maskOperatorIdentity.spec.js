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

  // A sibling directory that merely starts with the home path is a different
  // directory, and rewriting it to `~2` produces a path that resolves to
  // something else entirely.
  it('should not rewrite a directory that merely starts with the home path', () => {
    expect(maskOperatorIdentity('/home/alice2/x', { username: 'bob', homePath: '/home/alice' }))
      .to.equal('/home/alice2/x');
    expect(maskOperatorIdentity('/home/alice.bak/x', { username: 'bob', homePath: '/home/alice' }))
      .to.equal('/home/alice.bak/x');
  });

  it('should still rewrite the home path itself and its children', () => {
    expect(maskOperatorIdentity('/home/alice', { username: 'bob', homePath: '/home/alice' }))
      .to.equal('~');
    expect(maskOperatorIdentity('at /home/alice, then', { username: 'bob', homePath: '/home/alice' }))
      .to.equal('at ~, then');
  });

  // macOS and Windows resolve paths case-insensitively, so the same directory
  // reaches a report in more than one spelling.
  it('should rewrite the home path whatever case it arrives in', () => {
    expect(maskOperatorIdentity('/Home/Alice/x', { username: 'bob', homePath: '/home/alice' }))
      .to.equal('~/x');
  });

  it('should leave text alone when no identity is known', () => {
    expect(maskOperatorIdentity('/home/alice/x', { username: null, homePath: null }))
      .to.equal('/home/alice/x');
  });

  it('should pass through anything that is not a string', () => {
    expect(maskOperatorIdentity(42, { username: 'alice', homePath: '/home/alice' })).to.equal(42);
  });

  // Where the result has to stay an absolute path, the home directory can be
  // written as one
  it('should write the home directory with the replacement it is given', () => {
    expect(maskOperatorIdentity(
      '/home/alice/.dashmate/logs/core.log',
      { username: 'bob', homePath: '/home/alice' },
      { homeReplacement: '/~' },
    )).to.equal('/~/.dashmate/logs/core.log');
  });

  it('should mask every home directory it is given, one inside another included', () => {
    expect(maskOperatorIdentity(
      'a /srv/dash-data/x b /home/alice/y c /home/alice/nested/z',
      { username: 'bob', homePath: '/home/alice', homePaths: ['/home/alice/nested', '/srv/dash-data', '/home/alice'] },
    )).to.equal('a ~/x b ~/y c ~/z');
  });

  // A service account can have the root of the file system as its home, and
  // masking that would rewrite every path and URL in the text
  it('should not treat the root of the file system as a home directory', () => {
    expect(maskOperatorIdentity(
      'https://example.com/a /var/log/b',
      { username: 'bob', homePath: '/', homePaths: ['/', 'C:\\'] },
    )).to.equal('https://example.com/a /var/log/b');
  });

  it('should mask a home directory given with a trailing separator', () => {
    expect(maskOperatorIdentity('/home/alice/x', { username: 'bob', homePaths: ['/home/alice/'] }))
      .to.equal('~/x');
  });
});
