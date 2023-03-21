const { default: Ajv } = require('ajv/dist/2020');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

// const DashPlatformProtocol = require('@dashevo/dpp/lib/DashPlatformProtocol');
const createStateRepositoryMock = require('../../lib/test/mocks/createStateRepositoryMock');
let { DashPlatformProtocol } = require('../..');
const { default: loadWasmDpp } = require('../..');
// const JsonSchemaValidator = require('@dashevo/dpp/lib/validation/JsonSchemaValidator');

describe('DashPlatformProtocol', () => {
  let dpp;
  let stateRepositoryMock;
  let jsonSchemaValidatorMock;

  beforeEach(async function beforeEach() {
    ({ DashPlatformProtocol } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    jsonSchemaValidatorMock = {};

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
      jsonSchemaValidator: jsonSchemaValidatorMock,
    });
    await dpp.initialize();
  });

  describe('constructor', () => {
    it('should set default protocol version', () => {
      dpp = new DashPlatformProtocol();

      expect(dpp.getProtocolVersion()).to.equal(protocolVersion.latestVersion);
    });
  });

  describe('getStateRepository', () => {
    it('should return StateRepository', () => {
      const result = dpp.getStateRepository();

      expect(result).to.equal(stateRepositoryMock);
    });
  });

  describe('getJsonSchemaValidator', () => {
    it('should return JsonSchemaValidator', () => {
      const result = dpp.getJsonSchemaValidator();

      expect(result).to.equal(jsonSchemaValidatorMock);
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
      expect(dpp.getProtocolVersion()).to.equal(protocolVersion.latestVersion);

      dpp.setProtocolVersion(42);

      expect(dpp.getProtocolVersion()).to.equal(42);
    });
  });
});
