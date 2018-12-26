const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');
const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');

const validateSTPacketFactory = require('../../../../lib/stPacket/validation/validateSTPacketFactory');

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../lib/test/expect/expectError');

const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketFactory', () => {
  let rawSTPacket;
  let rawDapContract;
  let dapContract;
  let rawDapObjects;
  let validateSTPacket;
  let validateSTPacketDapContractsMock;
  let validateSTPacketDapObjectsMock;

  beforeEach(function beforeEach() {
    dapContract = getDapContractFixture();
    rawDapContract = dapContract.toJSON();
    rawDapObjects = getDapObjectsFixture().map(o => o.toJSON());
    rawSTPacket = {
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      objects: rawDapObjects,
    };

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateSTPacketDapContractsMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validateSTPacketDapObjectsMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketDapContractsMock,
      validateSTPacketDapObjectsMock,
    );
  });

  describe('contractId', () => {
    it('should be present', () => {
      delete rawSTPacket.contractId;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('contractId');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should be a string', () => {
      rawSTPacket.contractId = 1;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('type');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be less than 64 chars', () => {
      rawSTPacket.contractId = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('minLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be longer than 64 chars', () => {
      rawSTPacket.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('maxLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });
  });

  describe('itemsMerkleRoot', () => {
    it('should be present', () => {
      delete rawSTPacket.itemsMerkleRoot;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('itemsMerkleRoot');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsMerkleRoot = 1;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('type');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be less than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('minLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be longer than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('maxLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });
  });

  describe('itemsHash', () => {
    it('should be present', () => {
      delete rawSTPacket.itemsHash;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('itemsHash');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsHash = 1;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('type');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be less than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('minLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not be longer than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('maxLength');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });
  });

  describe('objects', () => {
    it('should be present', () => {
      delete rawSTPacket.objects;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('objects');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should be an array', () => {
      rawSTPacket.objects = 1;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.objects');
      expect(error.keyword).to.be.equal('type');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not contain more than 1000 items', () => {
      const thousandDapObjects = (new Array(1001)).fill(rawDapObjects[0]);
      rawSTPacket.objects.push(...thousandDapObjects);

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors).to.be.an('array').and.lengthOf(3);

      expect(errors[0].dataPath).to.be.equal('.objects');
      expect(errors[0].keyword).to.be.equal('maxItems');

      expect(errors[1].dataPath).to.be.equal('.objects');
      expect(errors[1].keyword).to.be.equal('maxItems');

      expect(errors[2].dataPath).to.be.equal('');
      expect(errors[2].keyword).to.be.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });
  });

  describe('contracts', () => {
    it('should be present', () => {
      delete rawSTPacket.contracts;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('contracts');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should be an array', () => {
      rawSTPacket.contracts = 1;

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contracts');
      expect(error.keyword).to.be.equal('type');

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });

    it('should not contain more than one contract', () => {
      rawSTPacket.contracts.push(rawDapContract, rawDapContract);

      const result = validateSTPacket(rawSTPacket, dapContract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors[0].dataPath).to.be.equal('.objects');
      expect(errors[0].keyword).to.be.equal('maxItems');

      expect(errors[1].dataPath).to.be.equal('.contracts');
      expect(errors[1].keyword).to.be.equal('maxItems');

      expect(errors[2].dataPath).to.be.equal('');
      expect(errors[2].keyword).to.be.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketDapContractsMock).to.be.not.called();
      expect(validateSTPacketDapObjectsMock).to.be.not.called();
    });
  });

  it('should return invalid result if packet is empty', () => {
    rawSTPacket.contracts = [];
    rawSTPacket.objects = [];

    const result = validateSTPacket(rawSTPacket, dapContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.keyword).to.be.equal('oneOf');
    expect(error.params.passingSchemas).to.be.deep.equal([0, 1]);

    expect(validateSTPacketDapContractsMock).to.be.not.called();
    expect(validateSTPacketDapObjectsMock).to.be.not.called();
  });

  it('should return invalid result if packet contains the both objects and contracts', () => {
    rawSTPacket.contracts.push(rawDapContract);

    const result = validateSTPacket(rawSTPacket, dapContract);

    expectJsonSchemaError(result, 3);

    const errors = result.getErrors();

    expect(errors[0].dataPath).to.be.equal('.objects');
    expect(errors[0].keyword).to.be.equal('maxItems');

    expect(errors[1].dataPath).to.be.equal('.contracts');
    expect(errors[1].keyword).to.be.equal('maxItems');

    expect(errors[2].dataPath).to.be.equal('');
    expect(errors[2].keyword).to.be.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();

    expect(validateSTPacketDapContractsMock).to.be.not.called();
    expect(validateSTPacketDapObjectsMock).to.be.not.called();
  });

  it('should return invalid result if there are additional properties in the packet', () => {
    const additionalProperty = 'additionalStuff';

    rawSTPacket[additionalProperty] = {};

    const result = validateSTPacket(rawSTPacket, dapContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('additionalProperties');
    expect(error.params.additionalProperty).to.be.equal(additionalProperty);

    expect(validateSTPacketDapContractsMock).to.be.not.called();
    expect(validateSTPacketDapObjectsMock).to.be.not.called();
  });

  it('should validate DAP Contract if present', () => {
    rawSTPacket.contracts = [
      rawDapContract,
    ];

    rawSTPacket.objects = [];

    const dapContractError = new ConsensusError('test');

    validateSTPacketDapContractsMock.returns(
      new ValidationResult([dapContractError]),
    );

    const result = validateSTPacket(rawSTPacket);

    expectValidationError(result);

    expect(validateSTPacketDapContractsMock).to.be.calledOnceWith(rawSTPacket);

    const [error] = result.getErrors();

    expect(error).to.be.equal(dapContractError);
  });

  it('should validate DAP Objects if present', () => {
    const dapContractError = new ConsensusError('test');

    validateSTPacketDapObjectsMock.returns(
      new ValidationResult([dapContractError]),
    );

    const result = validateSTPacket(rawSTPacket, dapContract);

    expectValidationError(result);

    expect(validateSTPacketDapObjectsMock).to.be.calledOnceWith(
      rawSTPacket,
      dapContract,
    );

    const [error] = result.getErrors();

    expect(error).to.be.equal(dapContractError);
  });

  it('should return valid result if packet structure is correct', () => {
    const result = validateSTPacket(rawSTPacket);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
