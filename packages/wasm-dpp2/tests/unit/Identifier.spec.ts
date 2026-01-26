import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

let identifierBytes: Uint8Array;

describe('Identifier', () => {
  before(async () => {
    identifierBytes = Uint8Array.from([
      9, 40, 40, 237, 192, 129, 211, 186, 26, 84, 240, 67, 37, 155, 148, 19, 104, 242, 199, 24, 136,
      27, 6, 169, 211, 71, 136, 59, 33, 191, 227, 19,
    ]);
  });

  describe('constructor()', () => {
    it('should create Identifier from bytes in constructor', () => {
      const identifier = new wasm.Identifier(identifierBytes);

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });

    it('should create Identifier from base58 in constructor', () => {
      const identifier = new wasm.Identifier('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });

    it('should create Identifier from Identifier', () => {
      const identifier = wasm.Identifier.fromBytes(identifierBytes);
      const identifier2 = new wasm.Identifier(identifier);

      expect(identifier2.toBytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('fromBase58()', () => {
    it('should create Identifier from base58', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('fromBase64()', () => {
    it('should create Identifier from base64', () => {
      const identifier = wasm.Identifier.fromBase64('CSgo7cCB07oaVPBDJZuUE2jyxxiIGwap00eIOyG/4xM=');

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('fromHex()', () => {
    it('should create Identifier from hex', () => {
      const identifier = wasm.Identifier.fromHex(
        '092828edc081d3ba1a54f043259b941368f2c718881b06a9d347883b21bfe313',
      );

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('fromBytes()', () => {
    it('should create Identifier from bytes', () => {
      const identifier = wasm.Identifier.fromBytes(identifierBytes);

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });
  });

  describe('toBase58()', () => {
    it('should return identifier base58', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toBase58()).to.equal('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');
    });
  });

  describe('toBase64()', () => {
    it('should return identifier base64', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toBase64()).to.equal('CSgo7cCB07oaVPBDJZuUE2jyxxiIGwap00eIOyG/4xM=');
    });
  });

  describe('toHex()', () => {
    it('should return identifier hex', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toHex()).to.equal(
        '092828edc081d3ba1a54f043259b941368f2c718881b06a9d347883b21bfe313',
      );
    });
  });

  describe('toBytes()', () => {
    it('should return identifier bytes', () => {
      const identifier = wasm.Identifier.fromBase58('ckBqfQe7LU7vwrwXopyCB4n5phZShjA16BGhNGpsD5U');

      expect(identifier.toBytes()).to.deep.equal(identifierBytes);
    });
  });
});
