const crypto = require('crypto');

const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');

const protocolVersion = require('../../../../../../../lib/protocolVersion');

const validateDataContractCreateTransitionBasicFactory = require('../../../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/validation/basic/validateDataContractCreateTransitionBasicFactory');

const DataContractCreateTransition = require('../../../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../../../lib/errors/ConsensusError');

const InvalidDataContractIdError = require('../../../../../../../lib/errors/InvalidDataContractIdError');

describe('validateDataContractCreateTransitionBasicFactory', () => {
  let validateDataContractMock;
  let validateDataContractCreateTransitionBasic;
  let stateTransition;
  let rawStateTransition;
  let dataContract;
  let rawDataContract;

  beforeEach(async function beforeEach() {
    validateDataContractMock = this.sinonSandbox.stub();

    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    // eslint-disable-next-line max-len
    validateDataContractCreateTransitionBasic = validateDataContractCreateTransitionBasicFactory(
      jsonSchemaValidator,
      validateDataContractMock,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 0', async () => {
      rawStateTransition.type = 666;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(0);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('dataContract');
    });

    it('should be valid', async () => {
      const dataContractError = new ConsensusError('test');
      const dataContractResult = new ValidationResult([
        dataContractError,
      ]);

      validateDataContractMock.returns(dataContractResult);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(dataContractError);

      expect(validateDataContractMock.getCall(0).args).to.have.deep.members([rawDataContract]);
    });

    it('should return invalid result on invalid Data Contract id', async () => {
      const dataContractResult = new ValidationResult();

      validateDataContractMock.returns(dataContractResult);

      rawStateTransition.dataContract.$id = crypto.randomBytes(34);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(InvalidDataContractIdError);
      expect(error.getRawDataContract()).to.equal(rawStateTransition.dataContract);
    });
  });

  describe('entropy', () => {
    it('should be present', async () => {
      delete rawStateTransition.entropy;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('entropy');
    });

    it('should be a byte array', async () => {
      rawStateTransition.entropy = new Array(32).fill('string');

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/entropy/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(31);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/entropy');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(32);
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(33);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/entropy');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(32);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/signature/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('minItems');
      expect(error.params.limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signature');
      expect(error.keyword).to.equal('maxItems');
      expect(error.params.limit).to.equal(65);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.keyword).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContractMock.returns(dataContractResult);

    const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractMock).to.be.calledOnceWith(rawDataContract);
  });
});
