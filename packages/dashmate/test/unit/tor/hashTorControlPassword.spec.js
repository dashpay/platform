import { expect } from 'chai';
import hashTorControlPassword from '../../../src/tor/hashTorControlPassword.js';

describe('hashTorControlPassword', () => {
  it('should produce the same hash as tor --hash-password for a known salt', () => {
    // Recorded from `tor --hash-password dashmatetest` (tor 0.4.9.11); the salt
    // is the first 8 bytes of its output.
    const salt = Buffer.from('A0C79FFFBAA5E730', 'hex');

    expect(hashTorControlPassword('dashmatetest', salt))
      .to.equal('16:A0C79FFFBAA5E73060ADD4E562A69F07771416D5EA11711DD1806D1901');
  });

  it('should use a fresh salt by default', () => {
    const first = hashTorControlPassword('password');
    const second = hashTorControlPassword('password');

    expect(first).to.match(/^16:[0-9A-F]{16}60[0-9A-F]{40}$/);
    expect(first).to.not.equal(second);
  });

  it('should reject a salt of the wrong length', () => {
    expect(() => hashTorControlPassword('password', Buffer.alloc(4))).to.throw('8 bytes');
  });
});
