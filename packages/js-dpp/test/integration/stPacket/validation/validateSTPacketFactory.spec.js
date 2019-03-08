const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');
const getSTPacketFixture = require('../../../../lib/test/fixtures/getSTPacketFixture');

const validateSTPacketFactory = require('../../../../lib/stPacket/validation/validateSTPacketFactory');

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../lib/test/expect/expectError');

const InvalidItemsMerkleRootError = require('../../../../lib/errors/InvalidItemsMerkleRootError');
const InvalidItemsHashError = require('../../../../lib/errors/InvalidItemsHashError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketFactory', () => {
  let stPacket;
  let rawSTPacket;
  let rawDPContract;
  let dpContract;
  let validateSTPacket;
  let validateSTPacketDPContractsMock;
  let validateSTPacketDPObjectsMock;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();
    rawDPContract = dpContract.toJSON();

    stPacket = getSTPacketFixture();
    rawSTPacket = stPacket.toJSON();

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateSTPacketDPContractsMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validateSTPacketDPObjectsMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketDPContractsMock,
      validateSTPacketDPObjectsMock,
    );
  });

  describe('contractId', () => {
    it('should be present', () => {
      delete rawSTPacket.contractId;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('contractId');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.contractId = 1;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no less than 64 chars', () => {
      rawSTPacket.contractId = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no longer than 64 chars', () => {
      rawSTPacket.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });
  });

  describe('itemsMerkleRoot', () => {
    it('should be present', () => {
      delete rawSTPacket.itemsMerkleRoot;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsMerkleRoot');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsMerkleRoot = 1;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no less than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no longer than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be merkle root of items', () => {
      rawSTPacket.itemsMerkleRoot = '8dsjd9w86b273ff86b273ff86b273ff86b3273ff86b273ff86b2737dh7ff86b2';

      const result = validateSTPacket(rawSTPacket);

      expectValidationError(result, InvalidItemsMerkleRootError);

      const [error] = result.getErrors();

      expect(error.getRawSTPacket()).to.equal(rawSTPacket);
    });
  });

  describe('itemsHash', () => {
    it('should be present', () => {
      delete rawSTPacket.itemsHash;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsHash');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsHash = 1;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no less than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be no longer than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be hash of items\' hashes', () => {
      rawSTPacket.itemsHash = '8dsjd9w86b273ff86b273ff86b273ff86b3273ff86b273ff86b2737dh7ff86b2';

      const result = validateSTPacket(rawSTPacket);

      expectValidationError(result, InvalidItemsHashError);

      const [error] = result.getErrors();

      expect(error.getRawSTPacket()).to.equal(rawSTPacket);
    });
  });

  describe('objects', () => {
    it('should be present', () => {
      delete rawSTPacket.objects;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('objects');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be an array', () => {
      rawSTPacket.objects = 1;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.objects');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should contain no more than 1000 items', () => {
      const thousandDPObjects = (new Array(1001)).fill(rawSTPacket.objects[0]);
      rawSTPacket.objects.push(...thousandDPObjects);

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors).to.be.an('array').with.lengthOf(3);

      expect(errors[0].dataPath).to.equal('.objects');
      expect(errors[0].keyword).to.equal('maxItems');

      expect(errors[1].dataPath).to.equal('.objects');
      expect(errors[1].keyword).to.equal('maxItems');

      expect(errors[2].dataPath).to.equal('');
      expect(errors[2].keyword).to.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });
  });

  describe('contracts', () => {
    it('should be present', () => {
      delete rawSTPacket.contracts;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('contracts');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should be an array', () => {
      rawSTPacket.contracts = 1;

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contracts');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });

    it('should contain no more than one contract', () => {
      rawSTPacket.contracts.push(rawDPContract, rawDPContract);

      const result = validateSTPacket(rawSTPacket, dpContract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors[0].dataPath).to.equal('.objects');
      expect(errors[0].keyword).to.equal('maxItems');

      expect(errors[1].dataPath).to.equal('.contracts');
      expect(errors[1].keyword).to.equal('maxItems');

      expect(errors[2].dataPath).to.equal('');
      expect(errors[2].keyword).to.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketDPContractsMock).to.have.not.been.called();
      expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
    });
  });

  it('should return invalid result if packet is empty', () => {
    rawSTPacket.contracts = [];
    rawSTPacket.objects = [];

    const result = validateSTPacket(rawSTPacket, dpContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.keyword).to.equal('oneOf');
    expect(error.params.passingSchemas).to.deep.equal([0, 1]);

    expect(validateSTPacketDPContractsMock).to.have.not.been.called();
    expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
  });

  it('should return invalid result if packet contains the both objects and contracts', () => {
    rawSTPacket.contracts.push(rawDPContract);

    const result = validateSTPacket(rawSTPacket, dpContract);

    expectJsonSchemaError(result, 3);

    const errors = result.getErrors();

    expect(errors[0].dataPath).to.equal('.objects');
    expect(errors[0].keyword).to.equal('maxItems');

    expect(errors[1].dataPath).to.equal('.contracts');
    expect(errors[1].keyword).to.equal('maxItems');

    expect(errors[2].dataPath).to.equal('');
    expect(errors[2].keyword).to.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();

    expect(validateSTPacketDPContractsMock).to.have.not.been.called();
    expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
  });

  it('should return invalid result if there are additional properties in the packet', () => {
    const additionalProperty = 'additionalStuff';

    rawSTPacket[additionalProperty] = {};

    const result = validateSTPacket(rawSTPacket, dpContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('additionalProperties');
    expect(error.params.additionalProperty).to.equal(additionalProperty);

    expect(validateSTPacketDPContractsMock).to.have.not.been.called();
    expect(validateSTPacketDPObjectsMock).to.have.not.been.called();
  });

  it('should validate DP Contract if present', () => {
    stPacket.setDPObjects([]);
    stPacket.setDPContract(dpContract);

    rawSTPacket = stPacket.toJSON();

    const dpContractError = new ConsensusError('test');

    validateSTPacketDPContractsMock.returns(
      new ValidationResult([dpContractError]),
    );

    const result = validateSTPacket(rawSTPacket);

    expectValidationError(result);

    expect(validateSTPacketDPContractsMock).to.have.been.calledOnceWith(rawSTPacket);

    const [error] = result.getErrors();

    expect(error).to.equal(dpContractError);
  });

  it('should validate DP Objects if present', () => {
    const dpContractError = new ConsensusError('test');

    validateSTPacketDPObjectsMock.returns(
      new ValidationResult([dpContractError]),
    );

    const result = validateSTPacket(rawSTPacket, dpContract);

    expectValidationError(result);

    expect(validateSTPacketDPObjectsMock).to.have.been.calledOnceWith(
      rawSTPacket,
      dpContract,
    );

    const [error] = result.getErrors();

    expect(error).to.equal(dpContractError);
  });

  it('should return valid result if packet structure is correct', () => {
    const result = validateSTPacket(rawSTPacket);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
