const crypto = require('crypto');
const bs58 = require('bs58');
let { Identifier, IdentifierError } = require('../..');
const { default: loadWasmDpp } = require('../..');

describe('Identifier', () => {
  let buffer;

  beforeEach(async () => {
    buffer = crypto.randomBytes(32);

    ({ Identifier, IdentifierError } = await loadWasmDpp());
  });

  describe('#constructor', () => {
    it('should accept Buffer', () => {
      const identifier = new Identifier(buffer);

      expect(identifier.toBytes()).to.be.deep.equal(new Uint8Array(buffer));
      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier).to.be.an.instanceOf(Uint8Array);
    });

    it('should accept Uint8Array', () => {
      const uint8 = new Uint8Array(buffer);
      const identifier = new Identifier(uint8);

      expect(identifier.toBytes()).to.be.deep.equal(uint8);
      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier).to.be.an.instanceOf(Uint8Array);
    });

    it('should throw error if first argument is not Uint8Array', () => {
      try {
        // eslint-disable-next-line no-unused-vars
        const id = new Identifier(1);

        expect.fail('Expected to throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(IdentifierError);
        expect(e.toString()).to.be.equal('IdentifierError: Identifier expects Uint8Array');
      }
    });

    it('should throw error if buffer is not 32 bytes long', () => {
      try {
        // eslint-disable-next-line no-unused-vars
        const identifier = new Identifier(Buffer.alloc(30));

        expect.fail('Expected to throw error');
      } catch (e) {
        expect(e).to.be.instanceOf(IdentifierError);
        expect(e.toString()).to.equal('IdentifierError: Identifier must be 32 long');
      }
    });
  });

  describe('#toBytes', () => {
    it('should return a new Uint8Array copy', () => {
      const identifier = new Identifier(buffer);

      const bytes = identifier.toBytes();
      expect(bytes).to.be.an.instanceOf(Uint8Array);
      expect(bytes).to.deep.equal(new Uint8Array(buffer));
      // mutating the returned copy must not affect the identifier
      bytes[0] = (bytes[0] + 1) & 0xff;
      expect(identifier.toBytes()[0]).to.equal(buffer[0]);
    });

    it('should be isolated from later mutations to the source bytes', () => {
      const source = new Uint8Array(buffer);
      const identifier = new Identifier(source);

      const originalByte = source[0];
      source[0] = (source[0] + 1) & 0xff;

      // The constructor must have copied the bytes; mutating the source
      // after construction does not leak into the identifier.
      expect(identifier.toBytes()[0]).to.equal(originalByte);
    });
  });

  describe('#toBuffer', () => {
    it('should return a new normal Buffer (deprecated)', () => {
      const identifier = new Identifier(buffer);

      const buf = identifier.toBuffer();
      expect(Buffer.isBuffer(buf)).to.equal(true);
      expect(buf).to.deep.equal(buffer);
    });
  });

  describe('#toJSON', () => {
    it('should return a base58 encoded string', () => {
      const identifier = new Identifier(buffer);

      const string = identifier.toJSON();

      expect(string).to.equal(bs58.encode(buffer));
    });
  });

  describe('#toString', () => {
    it('should return a base58 encoded string by default', () => {
      const base58string = bs58.encode(buffer);

      const identifier = new Identifier(buffer);

      const string = identifier.toString();

      expect(string).to.equal(base58string);
    });

    it('should return a string encoded with specified encoding', () => {
      const identifier = new Identifier(buffer);

      const string = identifier.toString('base64');

      expect(string).to.equal(buffer.toString('base64'));
    });
  });

  describe('#from', () => {
    it('should create an instance from Buffer', async () => {
      const identifier = Identifier.from(buffer);

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier.toBytes()).to.deep.equal(new Uint8Array(buffer));
    });

    it('should create an instance from Uint8Array', async () => {
      const uint8 = new Uint8Array(buffer);

      const identifier = Identifier.from(uint8);

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier.toBytes()).to.deep.equal(uint8);
    });

    it('should throw error if buffer is passed among with encoding', () => {
      try {
        Identifier.from(buffer, 'base64');

        expect.fail('Expected to throw error');
      } catch (e) {
        expect(e).to.be.instanceOf(IdentifierError);
        expect(e.toString()).to.be.equal('IdentifierError: encoding accepted only with type string');
      }
    });

    it('should create an instance with a base58 string', () => {
      const string = bs58.encode(buffer);

      const identifier = Identifier.from(string);

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier.toBytes()).to.deep.equal(new Uint8Array(buffer));
    });

    it('should create an instance with a base64 string', () => {
      const string = buffer.toString('base64');

      const identifier = Identifier.from(string, 'base64');

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier.toBytes()).to.deep.equal(new Uint8Array(buffer));
    });
  });
});
