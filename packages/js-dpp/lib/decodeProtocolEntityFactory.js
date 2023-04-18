const varint = require('varint');
const ProtocolVersionParsingError = require('./errors/consensus/basic/decode/ProtocolVersionParsingError');
const SerializedObjectParsingError = require('./errors/consensus/basic/decode/SerializedObjectParsingError');

const { decode } = require('./util/serializer');

function decodeProtocolEntityFactory() {
  /**
   * @typedef {decodeProtocolEntity}
   * @param {Buffer} buffer
   * @return {[number, Object]}
   */
  function decodeProtocolEntity(buffer) {
    // Parse protocol version from the first 4 bytes
    let protocolVersion;
    try {
      protocolVersion = varint.decode(buffer);
    } catch (error) {
      const consensusError = new ProtocolVersionParsingError(error.message);

      consensusError.setParsingError(error);

      throw consensusError;
    }

    let rawEntity;
    try {
      rawEntity = decode(
        buffer.slice(varint.encodingLength(protocolVersion), buffer.length),
      );
    } catch (error) {
      const consensusError = new SerializedObjectParsingError(error.message);

      consensusError.setParsingError(error);

      throw consensusError;
    }

    return [protocolVersion, rawEntity];
  }

  return decodeProtocolEntity;
}

module.exports = decodeProtocolEntityFactory;
