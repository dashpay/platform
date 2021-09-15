const decodeProtocolEntityFactory = require('../../lib/decodeProtocolEntityFactory');
const ProtocolVersionParsingError = require('../../lib/errors/consensus/basic/decode/ProtocolVersionParsingError');
const SerializedObjectParsingError = require('../../lib/errors/consensus/basic/decode/SerializedObjectParsingError');

const { encode } = require('../../lib/util/serializer');

describe('decodeProtocolEntityFactory', () => {
  let decodeProtocolEntity;
  let versionCompatibilityMap;
  let parsedProtocolVersion;
  let entityBuffer;
  let protocolVersionBuffer;
  let rawEntity;
  let buffer;

  beforeEach(() => {
    parsedProtocolVersion = 0;

    protocolVersionBuffer = Buffer.alloc(4);
    protocolVersionBuffer.writeUInt32LE(parsedProtocolVersion, 0);

    rawEntity = { test: 'successful' };
    entityBuffer = encode(rawEntity);

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    versionCompatibilityMap = {
      0: 0,
      1: 0,
    };

    decodeProtocolEntity = decodeProtocolEntityFactory(
      versionCompatibilityMap,
    );
  });

  it('should throw ProtocolVersionParsingError if can\'t parse protocol version', () => {
    buffer = Buffer.alloc(0);

    try {
      decodeProtocolEntity(buffer);

      expect.fail('should throw ProtocolVersionParsingError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(ProtocolVersionParsingError);

      expect(e.getParsingError()).to.be.instanceOf(Error);
      expect(e.getCode()).to.equal(1000);
    }
  });

  it('should throw SerializedObjectParsingError if entity decoding fails', () => {
    entityBuffer = Buffer.alloc(5).fill(1);

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
