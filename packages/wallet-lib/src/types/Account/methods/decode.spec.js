import { expect } from 'chai';
import cbor from 'cbor';
import decode from './decode.js';

describe('Account - decode', function suite() {
  this.timeout(10000);
  const jsonObject = {
    string: 'string',
    list: ['a', 'b', 'c', 'd'],
    obj: {
      int: 1,
      boolean: true,
      theNull: null,
    },
  };
  const encodedJSON = cbor.encodeCanonical(jsonObject);

  it('should decode JSON with cbor', () => {
    const decoded = decode('cbor', encodedJSON);
    expect(decoded).to.deep.equal(jsonObject);
  });
});
