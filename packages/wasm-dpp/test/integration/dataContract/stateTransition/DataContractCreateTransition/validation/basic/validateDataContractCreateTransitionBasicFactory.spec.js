const crypto = require('crypto');

const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const { expectJsonSchemaError, expectValidationError, expectValueError } = require('../../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../..');
const { getLatestProtocolVersion } = require('../../../../../../..');

describe.skip('validateDataContractCreateTransitionBasicFactory', () => {
  let stateTransition;
  let rawStateTransition;
  let dataContract;
  let rawDataContract;

  let DataContractCreateTransition;
  let validateDataContractCreateTransitionBasic;
  let ValidationResult;
  let ValueError;
  let InvalidDataContractIdError;

  before(async () => {
    ({
      DataContractCreateTransition,
      validateDataContractCreateTransitionBasic,
      ValidationResult,
      ValueError,
      InvalidDataContractIdError,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();
    rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectValueError(result);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(ValueError);
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      const [error] = result.getErrors();
      expect(error).to.be.an.instanceOf(ValueError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 0', async () => {
      rawStateTransition.type = 666;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(0);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('dataContract');
    });

    it('should be valid', async () => {
      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);
      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result on invalid Data Contract id', async () => {
      const expectedId = Buffer.from(rawStateTransition.dataContract.$id);
      rawStateTransition.dataContract.$id = Buffer.from(crypto.randomBytes(32));

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectValidationError(result);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1011);
      expect(error.getExpectedId()).to.deep.equal(expectedId);
      expect(error.getInvalidId()).to.deep.equal(rawStateTransition.dataContract.$id);
      expect(error).to.be.an.instanceOf(InvalidDataContractIdError);
    });
  });

  describe('entropy', () => {
    it('should be present', async () => {
      delete rawStateTransition.entropy;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('entropy');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(31);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/entropy');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(32);
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(33);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/entropy');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().maxItems).to.equal(32);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('signature');
    });

    it('should be not less than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signature');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(65);
    });

    it('should be not longer than 96 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(97);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

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

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be < 0', async () => {
      rawStateTransition.signaturePublicKeyId = -1;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result, 1);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/signaturePublicKeyId');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  it('should return valid result', async () => {
    const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
