const { default: loadWasmDpp } = require('../../../..');

let {
  deserializeConsensusError,
  decodeProtocolEntity,
  ProtocolVersionParsingError,
} = require('../../../..');

describe('deserializeConsensusError', () => {
  before(async () => {
    ({
      deserializeConsensusError,
      decodeProtocolEntity,
      ProtocolVersionParsingError,
    } = await loadWasmDpp());
  });

  it('should deserialize consensus error', async () => {
    try {
      await decodeProtocolEntity(Buffer.alloc(0));
    } catch (consensusError) {
      expect(consensusError).to.be.instanceOf(ProtocolVersionParsingError);
      expect(consensusError.getCode()).to.equals(1000);
      expect(consensusError.message).to.equals('Can\'t read protocol version from serialized object: protocol version could not be decoded as a varint');

      const bytes = consensusError.serialize();

      const recoveredError = deserializeConsensusError(bytes);
      expect(recoveredError).to.be.instanceOf(ProtocolVersionParsingError);
      expect(recoveredError.getCode()).to.equals(1000);
      expect(recoveredError.message).to.equals('Can\'t read protocol version from serialized object: protocol version could not be decoded as a varint');
    }
  });
});
