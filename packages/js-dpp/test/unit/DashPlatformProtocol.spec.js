const Ajv = require('ajv');

const DashPlatformProtocol = require('../../lib/DashPlatformProtocol');
const JsonSchemaValidator = require('../../lib/validation/JsonSchemaValidator');

const createDataProviderMock = require('../../lib/test/mocks/createDataProviderMock');

describe('DashPlatformProtocol', () => {
  let dpp;
  let dataProviderMock;
  let jsonSchemaValidatorMock;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    jsonSchemaValidatorMock = {};

    dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
      jsonSchemaValidator: jsonSchemaValidatorMock,
    });
  });

  describe('constructor', () => {
    it('should create JsonSchemaValidator if not passed in options', () => {
      dpp = new DashPlatformProtocol();

      const jsonSchemaValidator = dpp.getJsonSchemaValidator();

      expect(jsonSchemaValidator).to.be.instanceOf(JsonSchemaValidator);
      expect(jsonSchemaValidator.ajv).to.be.instanceOf(Ajv);
    });
  });

  describe('getDataProvider', () => {
    it('should return DataProvider', () => {
      const result = dpp.getDataProvider();

      expect(result).to.equal(dataProviderMock);
    });
  });

  describe('getJsonSchemaValidator', () => {
    it('should return JsonSchemaValidator', () => {
      const result = dpp.getJsonSchemaValidator();

      expect(result).to.equal(jsonSchemaValidatorMock);
    });
  });
});
