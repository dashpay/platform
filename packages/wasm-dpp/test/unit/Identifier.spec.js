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

      expect(identifier.toBuffer()).to.be.deep.equal(buffer);
      expect(identifier).to.be.an.instanceOf(Identifier);
    });

    it('should throw error if first argument is not Buffer', () => {
      try {
        // eslint-disable-next-line no-unused-vars
        const id = new Identifier(1);

        expect.fail('Expected to throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(IdentifierError);
        expect(e.toString()).to.be.equal('IdentifierError: Identifier expects Buffer');
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

  describe('#toBuffer', () => {
    it('should return a new normal Buffer', () => {
      const identifier = new Identifier(buffer);

      expect(identifier.toBuffer()).to.deep.equal(buffer);
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
      expect(identifier.toBuffer()).to.deep.equal(buffer);
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
      expect(identifier.toBuffer()).to.deep.equal(buffer);
    });

    it('should create an instance with a base64 string', () => {
      const string = buffer.toString('base64');

      const identifier = Identifier.from(string, 'base64');

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier.toBuffer()).to.deep.equal(buffer);
    });
  });
});
