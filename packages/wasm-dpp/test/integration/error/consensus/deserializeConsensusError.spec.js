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

      const bytes = consensusError.serialize();

      const recoveredError = deserializeConsensusError(bytes);
      expect(recoveredError).to.be.instanceOf(ProtocolVersionParsingError);
      expect(recoveredError.getCode()).to.equals(1000);
    }
  });
});
