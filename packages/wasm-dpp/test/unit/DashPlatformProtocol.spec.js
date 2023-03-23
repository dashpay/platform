const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const createStateRepositoryMock = require('../../lib/test/mocks/createStateRepositoryMock');
const getBlsAdapterMock = require('../../lib/test/mocks/getBlsAdapterMock');
let { DashPlatformProtocol } = require('../..');
const { default: loadWasmDpp } = require('../..');

describe('DashPlatformProtocol', () => {
  let dpp;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    ({ DashPlatformProtocol } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    dpp = new DashPlatformProtocol(
      getBlsAdapterMock(),
      stateRepositoryMock,
      protocolVersion.latestVersion,
    );
  });

  describe('constructor', () => {
    it('should set default protocol version', () => {
      dpp = new DashPlatformProtocol();

      expect(dpp.protocolVersion).to.equal(protocolVersion.latestVersion);
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
      expect(dpp.protocolVersion).to.equal(protocolVersion.latestVersion);

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });

  describe('getProtocolVersion', () => {
    it('should get protocol version', () => {
      expect(dpp.protocolVersion).to.equal(protocolVersion.latestVersion);

      dpp.setProtocolVersion(42);

      expect(dpp.protocolVersion).to.equal(42);
    });
  });
});
