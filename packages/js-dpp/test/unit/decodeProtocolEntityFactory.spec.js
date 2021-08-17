const decodeProtocolEntityFactory = require('../../lib/decodeProtocolEntityFactory');
const ProtocolVersionParsingError = require('../../lib/errors/ProtocolVersionParsingError');
const UnsupportedProtocolVersionError = require('../../lib/errors/UnsupportedProtocolVersionError');
const CompatibleProtocolVersionIsNotDefinedError = require('../../lib/errors/CompatibleProtocolVersionIsNotDefinedError');
const IncompatibleProtocolVersionError = require('../../lib/errors/IncompatibleProtocolVersionError');
const SerializedObjectParsingError = require('../../lib/errors/SerializedObjectParsingError');

const { encode } = require('../../lib/util/serializer');

describe('decodeProtocolEntityFactory', () => {
  let currentProtocolVersion;
  let decodeProtocolEntity;
  let versionCompatibilityMap;
  let parsedProtocolVersion;
  let entityBuffer;
  let protocolVersionBuffer;
  let rawEntity;
  let buffer;

  beforeEach(() => {
    currentProtocolVersion = 1;
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
      decodeProtocolEntity(buffer, currentProtocolVersion);
    } catch (e) {
      expect(e).to.be.an.instanceOf(ProtocolVersionParsingError);

      expect(e.getPayload()).to.equal(buffer);
      expect(e.getParsingError()).to.be.instanceOf(Error);
    }
  });

  it('should throw UnsupportedProtocolVersionError if parsed version is higher than the current one', () => {
    parsedProtocolVersion = 2;

    protocolVersionBuffer = Buffer.alloc(4);
    protocolVersionBuffer.writeUInt32LE(parsedProtocolVersion, 0);

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    try {
      decodeProtocolEntity(buffer, currentProtocolVersion);
    } catch (e) {
      expect(e).to.be.an.instanceOf(UnsupportedProtocolVersionError);

      expect(e.getPayload()).to.equal(buffer);
      expect(e.getParsedProtocolVersion()).to.equal(parsedProtocolVersion);
      expect(e.getCurrentProtocolVersion()).to.equal(currentProtocolVersion);
    }
  });

  it('should throw CompatibleProtocolVersionIsNotDefinedError if compatible version is not'
    + ' defined for the current protocol version', () => {
    delete versionCompatibilityMap[currentProtocolVersion.toString()];

    try {
      decodeProtocolEntity(buffer, currentProtocolVersion);
    } catch (e) {
      expect(e).to.be.an.instanceOf(CompatibleProtocolVersionIsNotDefinedError);
    }
  });

  it('should throw IncompatibleProtocolVersionError if parsed version is lower than compatible one', () => {
    const minimalProtocolVersion = 1;

    parsedProtocolVersion = 0;
    currentProtocolVersion = 5;

    versionCompatibilityMap[currentProtocolVersion.toString()] = minimalProtocolVersion;

    protocolVersionBuffer = Buffer.alloc(4);
    protocolVersionBuffer.writeUInt32LE(parsedProtocolVersion, 0);

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    try {
      decodeProtocolEntity(buffer, currentProtocolVersion);
    } catch (e) {
      expect(e).to.be.an.instanceOf(IncompatibleProtocolVersionError);

      expect(e.getPayload()).to.equal(buffer);
      expect(e.getParsedProtocolVersion()).to.equal(parsedProtocolVersion);
      expect(e.getMinimalProtocolVersion()).to.equal(minimalProtocolVersion);
    }
  });

  it('should throw SerializedObjectParsingError if entity decoding fails', () => {
    entityBuffer = Buffer.alloc(5).fill(1);

    buffer = Buffer.concat([protocolVersionBuffer, entityBuffer]);

    try {
      decodeProtocolEntity(buffer, currentProtocolVersion);
    } catch (e) {
      expect(e).to.be.an.instanceOf(SerializedObjectParsingError);

      expect(e.getPayload()).to.equal(buffer);
      expect(e.getParsingError()).to.be.an.instanceOf(Error);
    }
  });

  it('should decode protocol version and entity successfully', () => {
    const [protocolVersion, actualRawEntity] = decodeProtocolEntity(buffer, currentProtocolVersion);

    expect(protocolVersion).to.equal(parsedProtocolVersion);
    expect(rawEntity).to.deep.equal(actualRawEntity);
  });
});
