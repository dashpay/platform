const crypto = require('crypto');

const createAjv = require('../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../lib/validation/JsonSchemaValidator');

const DataContract = require('../../../../../lib/dataContract/DataContract');

const validateDataContractCreateTransitionStructureFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractCreateTransitionStructureFactory');

const DataContractCreateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractCreateTransition');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');

const InvalidIdentityPublicKeyTypeError = require('../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');
const InvalidDataContractIdError = require('../../../../../lib/errors/InvalidDataContractIdError');

describe('validateDataContractCreateTransitionStructureFactory', () => {
  let validateDataContractMock;
  let validateDataContractCreateTransitionStructure;
  let stateTransition;
  let rawStateTransition;
  let rawDataContract;
  let validateStateTransitionSignatureMock;
  let validateIdentityExistenceMock;

  beforeEach(function beforeEach() {
    validateDataContractMock = this.sinonSandbox.stub();

    const dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: DataContract.PROTOCOL_VERSION,
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();

    validateStateTransitionSignatureMock = this.sinonSandbox.stub();

    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(
      new ValidationResult(),
    );

    const ajv = createAjv();
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    // eslint-disable-next-line max-len
    validateDataContractCreateTransitionStructure = validateDataContractCreateTransitionStructureFactory(
      jsonSchemaValidator,
      validateDataContractMock,
      validateStateTransitionSignatureMock,
      validateIdentityExistenceMock,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 0', async () => {
      rawStateTransition.type = 666;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(0);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('dataContract');
    });

    it('should be valid', async () => {
      const dataContractError = new ConsensusError('test');
      const dataContractResult = new ValidationResult([
        dataContractError,
      ]);

      validateDataContractMock.returns(dataContractResult);

      const validateSignatureResult = new ValidationResult();
      validateStateTransitionSignatureMock.resolves(validateSignatureResult);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(dataContractError);

      expect(validateDataContractMock.getCall(0).args).to.have.deep.members([rawDataContract]);

      expect(validateStateTransitionSignatureMock).to.be.not.called();

      expect(validateIdentityExistenceMock).to.be.not.called();
    });

    it('should return invalid result on invalid Data Contract id', async () => {
      const dataContractResult = new ValidationResult();

      validateDataContractMock.returns(dataContractResult);

      const validateSignatureResult = new ValidationResult();
      validateStateTransitionSignatureMock.resolves(validateSignatureResult);

      rawStateTransition.dataContract.$id = crypto.randomBytes(34);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(InvalidDataContractIdError);
      expect(error.getRawDataContract()).to.equal(rawStateTransition.dataContract);
    });

    it('should return invalid result if Data Contract Identity is invalid', async () => {
      const dataContractResult = new ValidationResult();
      validateDataContractMock.returns(dataContractResult);

      const validateSignatureResult = new ValidationResult();
      validateStateTransitionSignatureMock.resolves(validateSignatureResult);

      const blockchainUserError = new ConsensusError('error');

      validateIdentityExistenceMock.resolves(
        new ValidationResult([blockchainUserError]),
      );

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(blockchainUserError);

      expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
        rawDataContract.ownerId,
      );
    });
  });

  describe('entropy', () => {
    it('should be present', async () => {
      delete rawStateTransition.entropy;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('entropy');
    });

    it('should be a binary', async () => {
      rawStateTransition.entropy = {};

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.entropy');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should be no less than 20 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(19);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.entropy');
      expect(error.keyword).to.equal('minBytesLength');
      expect(error.params.limit).to.equal(20);
    });

    it('should be no longer than 35 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(36);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.entropy');
      expect(error.keyword).to.equal('maxBytesLength');
      expect(error.params.limit).to.equal(35);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = {};

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('minBytesLength');
      expect(error.params.limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('maxBytesLength');
      expect(error.params.limit).to.equal(65);
    });

    it('should be valid', async () => {
      const dataContractResult = new ValidationResult();

      validateDataContractMock.returns(dataContractResult);

      const type = 1;
      const validationError = new InvalidIdentityPublicKeyTypeError(type);

      const validateSignatureResult = new ValidationResult([
        validationError,
      ]);

      validateStateTransitionSignatureMock.resolves(validateSignatureResult);

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(validationError);

      expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
        stateTransition,
        rawDataContract.ownerId,
      );

      expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
        rawDataContract.ownerId,
      );
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signaturePublicKeyId');
      expect(error.keyword).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signaturePublicKeyId');
      expect(error.keyword).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContractMock.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractMock).to.be.calledOnceWith(rawDataContract);

    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
      stateTransition,
      rawDataContract.ownerId,
    );

    stateTransition = new DataContractCreateTransition(rawStateTransition);

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
      rawDataContract.ownerId,
    );
  });
});
