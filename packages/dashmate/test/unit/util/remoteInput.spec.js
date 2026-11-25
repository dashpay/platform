import { sanitizeForLog } from '../../../src/util/remoteInput.js';

describe('remoteInput', () => {
  describe('sanitizeForLog', () => {
    // Every character here is invisible or reorders the text after it, so remote text can
    // use it to disguise what an operator reads. They are built from code points so that
    // nothing invisible is written into this file.
    const UNSAFE_CHARACTERS = {
      ESC: 0x1b,
      DEL: 0x7f,
      CSI: 0x9b,
      'ARABIC LETTER MARK': 0x061c,
      'ZERO WIDTH SPACE': 0x200b,
      'LINE SEPARATOR': 0x2028,
      'RIGHT-TO-LEFT OVERRIDE': 0x202e,
      'LEFT-TO-RIGHT ISOLATE': 0x2066,
      'POP DIRECTIONAL ISOLATE': 0x2069,
      'ZERO WIDTH NO-BREAK SPACE': 0xfeff,
    };

    Object.entries(UNSAFE_CHARACTERS).forEach(([name, codePoint]) => {
      it(`should remove ${name}`, () => {
        const character = String.fromCharCode(codePoint);

        const sanitized = sanitizeForLog(`apt${character}-get failed`);

        expect(sanitized).to.not.contain(character);
        expect(sanitized).to.contain('failed');
      });
    });

    it('should keep ordinary text as it is', () => {
      expect(sanitizeForLog('E: Could not get lock /var/lib/dpkg/lock'))
        .to.equal('E: Could not get lock /var/lib/dpkg/lock');
    });

    it('should report a value that is not text instead of coercing it', () => {
      expect(sanitizeForLog({ toString: 1 })).to.equal('[unprintable]');
    });
  });
});
