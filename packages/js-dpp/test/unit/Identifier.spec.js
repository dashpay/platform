const crypto = require('crypto');
const bs58 = require('bs58');
const Identifier = require('../../lib/identifier/Identifier');
const IdentifierError = require('../../lib/identifier/errors/IdentifierError');

describe('Identifier', () => {
  let buffer;

  beforeEach(() => {
    buffer = crypto.randomBytes(32);
  });

  describe('#constructor', () => {
    it('should accept Buffer', () => {
      const identifier = new Identifier(buffer);

      expect(identifier).to.be.deep.equal(buffer);
      expect(identifier).to.be.an.instanceOf(Identifier);
    });

    it('should throw error if first argument is not Buffer', () => {
      expect(
        () => new Identifier(1),
      ).to.throw(IdentifierError, 'Identifier expects Buffer');
    });

    it('should throw error if buffer is not 32 bytes long', () => {
      expect(
        () => new Identifier(Buffer.alloc(30)),
      ).to.throw(IdentifierError, 'Identifier must be 32 long');
    });
  });

  describe('#toBuffer', () => {
    it('should return a new normal Buffer', () => {
      const identifier = new Identifier(buffer);

      expect(identifier.toBuffer()).to.deep.equal(buffer);
    });
  });

  describe('#encodeCBOR', () => {
    let encoderMock;

    beforeEach(function before() {
      encoderMock = {
        pushAny: this.sinonSandbox.stub(),
      };
    });

    it('should encode using cbor encoder', () => {
      const identifier = new Identifier(buffer);

      const result = identifier.encodeCBOR(encoderMock);

      expect(result).to.be.true();
      expect(encoderMock.pushAny).to.be.calledOnceWithExactly(buffer);
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
      expect(identifier).to.deep.equal(buffer);
    });

    it('should throw error if buffer is passed among with encoding', async () => {
      expect(
        () => Identifier.from(buffer, 'base64'),
      ).to.throw(IdentifierError, 'encoding accepted only with type string');
    });

    it('should create an instance with a base58 string', () => {
      const string = bs58.encode(buffer);

      const identifier = Identifier.from(string);

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier).to.deep.equal(buffer);
    });

    it('should create an instance with a base64 string', () => {
      const string = buffer.toString('base64');

      const identifier = Identifier.from(string, 'base64');

      expect(identifier).to.be.an.instanceOf(Identifier);
      expect(identifier).to.deep.equal(buffer);
    });
  });
});
