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

  describe('invisible and bidi characters', () => {
    // Each of these is invisible or reorders what follows it, so a registry can
    // use them to hide text in a message an operator is reading to decide what
    // to do next. They are named by code point because writing them literally
    // makes this file itself unreadable.
    const HIDDEN = {
      ESC: 0x1b,
      CR: 0x0d,
      DEL: 0x7f,
      LRM: 0x200e,
      ALM: 0x061c,
      RLO: 0x202e,
      ZWSP: 0x200b,
      ZWJ: 0x200d,
      LS: 0x2028,
      PS: 0x2029,
      BOM: 0xfeff,
    };

    Object.entries(HIDDEN).forEach(([name, codePoint]) => {
      it(`should not let a registry hide text behind ${name}`, () => {
        const text = `left${String.fromCodePoint(codePoint)}right`;

        expect(sanitizeRemoteText(text)).to.equal('left right');
      });
    });
  });
});
