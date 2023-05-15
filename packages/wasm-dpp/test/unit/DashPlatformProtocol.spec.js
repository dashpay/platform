const crypto = require('crypto');

const createStateRepositoryMock = require('../../lib/test/mocks/createStateRepositoryMock');
const getBlsAdapterMock = require('../../lib/test/mocks/getBlsAdapterMock');
const { DashPlatformProtocol, getLatestProtocolVersion } = require('../..');
const { default: loadWasmDpp } = require('../..');

describe('DashPlatformProtocol', () => {
  let dpp;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    await loadWasmDpp();
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    dpp = new DashPlatformProtocol(
      getBlsAdapterMock(),
      stateRepositoryMock,
      { generate: () => crypto.randomBytes(32) },
      getLatestProtocolVersion(),
    );
  });

  describe('constructor', () => {
    it('should set default protocol version', () => {
      dpp = new DashPlatformProtocol();

      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());
    });
  });

  describe('getStateRepository', () => {
    it('should return StateRepository', () => {
      const result = dpp.getStateRepository();

      expect(result).to.equal(stateRepositoryMock);
    });
  });

  describe('setProtocolVersion', () => {
    it('should set protocol version', () => {
      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });

  describe('getProtocolVersion', () => {
    it('should get protocol version', () => {
      expect(dpp.protocolVersion).to.equal(getLatestProtocolVersion());

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });
});
