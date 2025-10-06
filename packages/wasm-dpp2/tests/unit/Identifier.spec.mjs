import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

let identifierBytes;

describe('Identifier', () => {
  before(async () => {
    identifierBytes = Uint8Array.from([9, 40, 40, 237, 192, 129, 211, 186, 26, 84, 240, 67, 37, 155, 148, 19, 104, 242, 199, 24, 136, 27, 6, 169, 211, 71, 136, 59, 33, 191, 227, 19]);
  });

  describe('serialization / deserialization', () => {
    it('should allows to create Identifier from base58', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from base64', () => {
      const identifier = wasm.Identifier.fromBase64('CSgo7cCB07oaVPBDJZuUE2jyxxiIGwap00eIOyG/4xM=');

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from hex', () => {
      const identifier = wasm.Identifier.fromHex('092828edc081d3ba1a54f043259b941368f2c718881b06a9d347883b21bfe313');

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from bytes', () => {
      const identifier = wasm.Identifier.fromBytes(identifierBytes);

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from Identifier', () => {
      const identifier = wasm.Identifier.fromBytes(identifierBytes);
      const identifier2 = new wasm.Identifier(identifier);

      expect(identifier2.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from bytes in constructor', () => {
      const identifier = new wasm.Identifier(identifierBytes);

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });

    it('should allows to create Identifier from base58 in constructor', () => {
      const identifier = new wasm.Identifier('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('getters', () => {
    it('should allow to get identifier base58', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.base58()).to.equal('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');
    });

    it('should allow to get identifier base64', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.base64()).to.equal('CSgo7cCB07oaVPBDJZuUE2jyxxiIGwap00eIOyG/4xM=');
    });

    it('should allow to get identifier hex', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toHex()).to.equal('092828edc081d3ba1a54f043259b941368f2c718881b06a9d347883b21bfe313');
    });

    it('should allow to get identifier bytes', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.bytes()).to.deep.equal(identifierBytes);
    });
  });
});
