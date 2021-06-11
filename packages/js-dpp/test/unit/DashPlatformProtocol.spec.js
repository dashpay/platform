const { default: Ajv } = require('ajv/dist/2020');

const DashPlatformProtocol = require('../../lib/DashPlatformProtocol');
const JsonSchemaValidator = require('../../lib/validation/JsonSchemaValidator');

const createStateRepositoryMock = require('../../lib/test/mocks/createStateRepositoryMock');

describe('DashPlatformProtocol', () => {
  let dpp;
  let stateRepositoryMock;
  let jsonSchemaValidatorMock;

  beforeEach(async function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    jsonSchemaValidatorMock = {};

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
      jsonSchemaValidator: jsonSchemaValidatorMock,
    });
    await dpp.initialize();
  });

  describe('constructor', () => {
    it('should create JsonSchemaValidator if not passed in options', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      const jsonSchemaValidator = dpp.getJsonSchemaValidator();

      expect(jsonSchemaValidator).to.be.instanceOf(JsonSchemaValidator);
      expect(jsonSchemaValidator.ajv).to.be.instanceOf(Ajv);
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
});
