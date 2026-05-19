import { expect } from 'chai';
import { getBytesOf } from './index.js';

describe('Utils - getBytesOf', function suite() {
  it('should have getBytesOf return false on unknown type', () => {
    expect(getBytesOf(null, 'foo')).to.be.equal(false);
  });
});
