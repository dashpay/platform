import sanitizeRemoteText from '../../../src/util/sanitizeRemoteText.js';

describe('sanitizeRemoteText', () => {
  it('should keep an ordinary registry message as it is', () => {
    expect(sanitizeRemoteText('toomanyrequests: You have reached your pull rate limit'))
      .to.equal('toomanyrequests: You have reached your pull rate limit');
  });

  // ESC starts every ANSI sequence, and one printed as it arrives can erase the
  // screen, move the cursor over lines already written or address the terminal
  it('should remove escape sequences', () => {
    const escape = String.fromCharCode(0x1b);

    const sanitized = sanitizeRemoteText(`${escape}[2J${escape}[1;1Hdenied`);

    expect(sanitized).to.not.contain(escape);
    expect(sanitized).to.contain('denied');
  });

  it('should collapse a message spread over several lines onto one', () => {
    expect(sanitizeRemoteText('denied\n\tby the registry')).to.equal('denied by the registry');
  });

  // An unbounded message pushes everything else out of the operator's scrollback
  it('should bound the length and say that it did', () => {
    const sanitized = sanitizeRemoteText('A'.repeat(5000));

    expect(sanitized).to.have.lengthOf.below(600);
    expect(sanitized).to.match(/^A+ \(truncated\)$/);
  });

  it('should pass through anything that is not a string', () => {
    expect(sanitizeRemoteText(undefined)).to.be.undefined();
    expect(sanitizeRemoteText(null)).to.equal(null);
  });
});
