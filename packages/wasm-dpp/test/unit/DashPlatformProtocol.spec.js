const crypto = require('crypto');

const { DashPlatformProtocol, getLatestProtocolVersion } = require('../..');
const { default: loadWasmDpp } = require('../..');

describe('DashPlatformProtocol', () => {
  let dpp;

  beforeEach(async () => {
    await loadWasmDpp();

    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
      getLatestProtocolVersion(),
    );
  });

  describe.skip('constructor', () => {
    it('should set default protocol version', () => {
      dpp = new DashPlatformProtocol();

      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());
    });
  });

  describe.skip('setProtocolVersion', () => {
    it('should set protocol version', () => {
      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });

  describe.skip('getProtocolVersion', () => {
    it('should get protocol version', () => {
      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });
});
