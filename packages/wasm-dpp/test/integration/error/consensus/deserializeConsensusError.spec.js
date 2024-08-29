const { default: loadWasmDpp } = require('../../../..');

let {
  deserializeConsensusError,
  ProtocolVersionParsingError,
} = require('../../../..');

describe('deserializeConsensusError', () => {
  before(async () => {
    ({
      deserializeConsensusError,
      ProtocolVersionParsingError,
    } = await loadWasmDpp());
  });

  it('should deserialize consensus error', async () => {
    const consensusError = new ProtocolVersionParsingError('test');
    const message = 'Can\'t read protocol version from serialized object: test';

    expect(consensusError).to.be.instanceOf(ProtocolVersionParsingError);
    expect(consensusError.getCode()).to.equals(10001);
    expect(consensusError.message).to.equals(message);

    const serializedConsensusError = consensusError.serialize();

    const recoveredError = deserializeConsensusError(serializedConsensusError);
    expect(recoveredError).to.be.instanceOf(ProtocolVersionParsingError);
    expect(recoveredError.getCode()).to.equals(10001);
    expect(recoveredError.message).to.equals(message);
  });
});
