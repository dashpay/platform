const crypto = require('crypto');
const bs58 = require('bs58');
const EncodedBuffer = require('../../../../lib/util/encoding/EncodedBuffer');
const InvalidBufferEncodingError = require('../../../../lib/errors/InvalidBufferEncodingError');

describe('EncodedBuffer', () => {
  let buffer;

  beforeEach(() => {
    buffer = crypto.randomBytes(8);
  });

  describe('#toString', () => {
    it('should return buffer encoded in base64 encoding without padding', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE64);

      const string = encodedBuffer.toString();

      expect(string).to.equal(buffer.toString('base64').replace(/=/g, ''));
    });

    it('should return buffer encoded in base58 encoding', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE58);

      const string = encodedBuffer.toString();

      expect(string).to.equal(bs58.encode(buffer));
    });
  });

  describe('#toBuffer', () => {
    it('should return a new normal Buffer', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE64);

      const data = encodedBuffer.toBuffer();

      expect(data).to.deep.equal(buffer);
    });
  });

  describe('#getEncoding', () => {
    it('should return encoding', () => {
      const encoding = EncodedBuffer.ENCODING.BASE64;
      const encodedBuffer = new EncodedBuffer(buffer, encoding);

      const data = encodedBuffer.getEncoding();

      expect(data).to.equal(encoding);
    });
  });

  describe('#encodeCBOR', () => {
    let encoderMock;

    beforeEach(function before() {
      encoderMock = {
        push: this.sinonSandbox.stub(),
      };
    });

    it('should encode using cbor encoder', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE64);

      const result = encodedBuffer.encodeCBOR(encoderMock);

      expect(result).to.be.true();
      expect(encoderMock.push).to.be.calledOnceWithExactly(buffer);
    });
  });

  describe('#from', () => {
    it('should create an instance using base64 string representation', () => {
      const encoding = EncodedBuffer.ENCODING.BASE64;
      const data = buffer.toString('base64').replace(/=/g, '');

      const encodedBuffer = EncodedBuffer.from(data, encoding);

      expect(encodedBuffer).to.be.an.instanceOf(EncodedBuffer);
      expect(encodedBuffer.toBuffer()).to.deep.equal(buffer);
      expect(encodedBuffer.getEncoding()).to.equal(encoding);
    });

    it('should create an instance using base58 string representation', () => {
      const encoding = EncodedBuffer.ENCODING.BASE58;
      const data = bs58.encode(buffer);

      const encodedBuffer = EncodedBuffer.from(data, encoding);

      expect(encodedBuffer).to.be.an.instanceOf(EncodedBuffer);
      expect(encodedBuffer.toBuffer()).to.deep.equal(buffer);
      expect(encodedBuffer.getEncoding()).to.equal(encoding);
    });

    it('should create an instance using buffer', async () => {
      const encoding = EncodedBuffer.ENCODING.BASE58;

      const encodedBuffer = EncodedBuffer.from(buffer, encoding);

      expect(encodedBuffer).to.be.an.instanceOf(EncodedBuffer);
      expect(encodedBuffer.toBuffer()).to.deep.equal(buffer);
      expect(encodedBuffer.getEncoding()).to.equal(encoding);
    });
  });

  describe('#toJSON', () => {
    it('should return buffer encoded in base64 encoding without padding', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE64);

      const string = encodedBuffer.toJSON();

      expect(string).to.equal(buffer.toString('base64').replace(/=/g, ''));
    });

    it('should return buffer encoded in base58 encoding', () => {
      const encodedBuffer = new EncodedBuffer(buffer, EncodedBuffer.ENCODING.BASE58);

      const string = encodedBuffer.toJSON();

      expect(string).to.equal(bs58.encode(buffer));
    });
  });

  it('should throw InvalidBufferEncodingError if encoding in unknown', () => {
    const encoding = 'myEncoding';

    try {
      // eslint-disable-next-line no-new
      new EncodedBuffer(buffer, encoding);

      expect.fail('Should throw InvalidBufferEncodingError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidBufferEncodingError);
      expect(e.getEncoding()).to.equal(encoding);
    }
  });
});
