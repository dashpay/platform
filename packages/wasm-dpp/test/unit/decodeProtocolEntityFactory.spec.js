const varint = require('varint');
const { encode } = require('@dashevo/dpp/lib/util/serializer');

let { decodeProtocolEntity, ProtocolVersionParsingError, SerializedObjectParsingError } = require('../..');
const { default: loadWasmDpp } = require('../..');

describe('decodeProtocolEntityFactory', () => {
  let parsedProtocolVersion;
  let entityBuffer;
  let protocolVersionBuffer;
  let rawEntity;
  let buffer;

  beforeEach(async () => {
    parsedProtocolVersion = 0;

    protocolVersionBuffer = Buffer.from(varint.encode(parsedProtocolVersion));

    rawEntity = { test: 'successful' };
    entityBuffer = encode(rawEntity);

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    ({
      decodeProtocolEntity,
      ProtocolVersionParsingError,
      SerializedObjectParsingError,
    } = await loadWasmDpp());
  });

  it('should throw ProtocolVersionParsingError if can\'t parse protocol version', () => {
    buffer = Buffer.alloc(0);

    try {
      decodeProtocolEntity(buffer);

      expect.fail('should throw ProtocolVersionParsingError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(ProtocolVersionParsingError);

      expect(e.getParsingError()).to.equals('protocol version could not be decoded as a varint');
      expect(e.getCode()).to.equal(1000);
    }
  });

  it('should throw SerializedObjectParsingError if entity decoding fails', () => {
    entityBuffer = Buffer.from('invalid');

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    try {
      decodeProtocolEntity(buffer);

      expect.fail('should throw SerializedObjectParsingError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(SerializedObjectParsingError);

      expect(e.getParsingError()).to.be.an.instanceOf(Error);
      expect(e.getCode()).to.equal(1001);
    }
  });

  it('should decode protocol version and entity successfully', () => {
    const [protocolVersion, actualRawEntity] = decodeProtocolEntity(buffer);

    expect(protocolVersion).to.equal(parsedProtocolVersion);
    expect(rawEntity).to.deep.equal(actualRawEntity);
  });
});
