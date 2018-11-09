const { encode, decode } = require('../../../lib/util/serializer');

describe('serializer', () => {
  it('should successfully encode and decode an object', () => {
    const data = {
      a: 1,
      b: 2,
      c: {
        x: 3,
        y: 4,
        z: [1, 2, 3],
      },
      $some: 'message',
    };

    const encoded = encode(data);

    expect(encoded).to.be.not.null();
    expect(encoded).to.be.not.deep.equal(data);

    const decoded = decode(encoded);

    expect(decoded).to.be.deep.equal(data);
  });
});
