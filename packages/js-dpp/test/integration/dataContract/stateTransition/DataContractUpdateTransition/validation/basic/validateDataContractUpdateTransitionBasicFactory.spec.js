const lodashClone = require('lodash.clonedeep');

const jsonPatch = require('fast-json-patch');
const jsonSchemaDiffValidator = require('json-schema-diff-validator');

const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');

const protocolVersion = require('../../../../../../../lib/version/protocolVersion');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const validateDataContractUpdateTransitionBasicFactory = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/validation/basic/validateDataContractUpdateTransitionBasicFactory');

const DataContractUpdateTransition = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/DataContractUpdateTransition');

const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');
const DataContractImmutablePropertiesUpdateError = require('../../../../../../../lib/errors/consensus/basic/dataContract/DataContractImmutablePropertiesUpdateError');
const IncompatibleDataContractSchemaError = require('../../../../../../../lib/errors/consensus/basic/dataContract/IncompatibleDataContractSchemaError');

describe('validateDataContractUpdateTransitionBasicFactory', () => {
  let validateDataContractMock;
  let validateDataContractUpdateTransitionBasic;
  let stateTransition;
  let rawStateTransition;
  let dataContract;
  let rawDataContract;
  let validateProtocolVersionMock;
  let validateIndicesAreNotChangedMock;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    validateDataContractMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    dataContract = getDataContractFixture();

    rawDataContract = lodashClone(dataContract.toObject());
    rawDataContract.version += 1;

    stateTransition = new DataContractUpdateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: rawDataContract,
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateIndicesAreNotChangedMock = this.sinonSandbox.stub();
    validateIndicesAreNotChangedMock.returns(new ValidationResult());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    // eslint-disable-next-line max-len
    validateDataContractUpdateTransitionBasic = validateDataContractUpdateTransitionBasicFactory(
      jsonSchemaValidator,
      validateDataContractMock,
      validateProtocolVersionMock,
      stateRepositoryMock,
      jsonSchemaDiffValidator,
      validateIndicesAreNotChangedMock,
      jsonPatch,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;

      const protocolVersionError = new SomeConsensusError('test');
      const protocolVersionResult = new ValidationResult([
        protocolVersionError,
      ]);

      validateProtocolVersionMock.returns(protocolVersionResult);

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectValidationError(result, SomeConsensusError);

      const [error] = result.getErrors();

      expect(error).to.equal(protocolVersionError);

      expect(validateProtocolVersionMock).to.be.calledOnceWith(
        rawStateTransition.protocolVersion,
      );
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 4', async () => {
      rawStateTransition.type = 666;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(4);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('dataContract');
    });

    it('should have backward compatible schema', async () => {
      rawStateTransition.dataContract.documents.indexedDocument = undefined;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(IncompatibleDataContractSchemaError);
      expect(error.getOperation()).to.equal('remove');
      expect(error.getFieldPath()).to.equal('/indexedDocument');
    });

    it('should not have immutable fields changed', async () => {
      rawStateTransition.dataContract.$schema = undefined;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractImmutablePropertiesUpdateError);
      expect(error.getOperation()).to.equal('remove');
      expect(error.getFieldPath()).to.equal('/$schema');
    });

    it('should be valid', async () => {
      const dataContractError = new SomeConsensusError('test');
      const dataContractResult = new ValidationResult([
        dataContractError,
      ]);

      validateDataContractMock.returns(dataContractResult);

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(dataContractError);

      expect(validateDataContractMock.getCall(0).args).to.have.deep.members([rawDataContract]);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().limit).to.equal(65);
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().limit).to.equal(65);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateDataContractUpdateTransitionBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractMock).to.be.calledOnceWith(rawDataContract);
  });
});
