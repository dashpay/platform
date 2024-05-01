const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const { expectJsonSchemaError, expectValidationError, expectValueError } = require('../../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../..');
const { getLatestProtocolVersion } = require('../../../../../../..');

describe.skip('validateDataContractUpdateTransitionBasicFactory', () => {
  let DataContractUpdateTransition;
  let validateDataContractUpdateTransitionBasic;
  let ValidationResult;
  let StateTransitionExecutionContext;
  let ValueError;
  let DataContractValidator;
  let DataContractFactory;
  let DataContractImmutablePropertiesUpdateError;
  let IncompatibleDataContractSchemaError;

  let validateStateTransition;
  let stateTransition;
  let rawStateTransition;
  let dataContract;
  let rawDataContract;
  let stateRepositoryMock;
  let executionContext;

  before(async () => {
    ({
      DataContractUpdateTransition,
      validateDataContractUpdateTransitionBasic,
      ValidationResult,
      StateTransitionExecutionContext,
      ValueError,
      DataContractValidator,
      DataContractFactory,
      DataContractImmutablePropertiesUpdateError,
      IncompatibleDataContractSchemaError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    dataContract = await getDataContractFixture();

    rawDataContract = dataContract.toObject();
    rawDataContract.version += 1;

    stateTransition = new DataContractUpdateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: rawDataContract,
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();

    const validator = new DataContractValidator();
    const dataContractFactory = new DataContractFactory(getLatestProtocolVersion(), validator);
    const reCreatedDataContract = await dataContractFactory
      .createFromBuffer(dataContract.toBuffer());
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(reCreatedDataContract);

    executionContext = new StateTransitionExecutionContext();

    validateStateTransition = (
      st,
      ctx,
    ) => validateDataContractUpdateTransitionBasic(stateRepositoryMock, st, ctx);
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectValueError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 4', async () => {
      rawStateTransition.type = 666;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(4);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('dataContract');
    });

    it('should have no existing documents removed', async () => {
      delete rawStateTransition.dataContract.documents.indexedDocument;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(IncompatibleDataContractSchemaError);
      expect(error.getOperation()).to.equal('remove json');
      expect(error.getFieldPath()).to.equal('/additionalProperties');
    });

    it('should allow making backward compatible changes to existing documents', async () => {
      rawStateTransition.dataContract.documents.indexedDocument.properties.newProp = {
        type: 'integer',
        minimum: 0,
      };

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      expect(result.isValid()).to.be.true();
    });

    it('should have existing documents schema backward compatible', async () => {
      rawStateTransition.dataContract.documents.indexedDocument.properties.firstName.maxLength = 4;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(IncompatibleDataContractSchemaError);
      expect(error.getOperation()).to.equal('replace json');
      expect(error.getFieldPath()).to.equal('/properties/firstName/maxLength');
    });

    it('should allow defining new document', async () => {
      rawStateTransition.dataContract.documents.myNewAwesomeDoc = {
        type: 'object',
        properties: {
          name: {
            type: 'string',
          },
        },
        required: ['name'],
        additionalProperties: false,
      };

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      expect(result.isValid()).to.be.true();
    });

    it('should not have root immutable properties changed', async () => {
      rawStateTransition.dataContract.ownerId = Buffer.alloc(32);

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractImmutablePropertiesUpdateError);
      expect(error.getOperation()).to.equal('replace');
      expect(error.getFieldPath()).to.equal('/ownerId');
    });

    it('should be valid', async () => {
      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      expect(result.isValid()).to.be.true();
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      // Because of how byte arrays are handled in input arguments it fails way before
      // any of validators come in so we cannot provide any sensible error, but at
      // least it won't work with bad type, so we're safe.
      try {
        await validateStateTransition(
          rawStateTransition,
          executionContext,
        );
        expect.fail('wasm bindgen error must be thrown');
      } catch (error) {
        expect(error.message).to.contain('invalid type: string "string", expected u8');
      }
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(65);
    });

    it('should be not longer than 96 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(97);

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().maxItems).to.equal(96);
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should be an integer', async () => {
      rawStateTransition.signaturePublicKeyId = 1.4;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateStateTransition(
        rawStateTransition,
        executionContext,
      );

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateStateTransition(
      rawStateTransition,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should not check Data Contract on dry run', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    executionContext.enableDryRun();

    const result = await validateStateTransition(
      rawStateTransition,
      executionContext,
    );

    executionContext.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
