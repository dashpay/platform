const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');
const getSTPacketFixture = require('../../../../lib/test/fixtures/getSTPacketFixture');

const validateSTPacketFactory = require('../../../../lib/stPacket/validation/validateSTPacketFactory');

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../lib/test/expect/expectError');

const InvalidItemsMerkleRootError = require('../../../../lib/errors/InvalidItemsMerkleRootError');
const InvalidItemsHashError = require('../../../../lib/errors/InvalidItemsHashError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe.skip('validateSTPacketFactory', () => {
  let stPacket;
  let rawSTPacket;
  let rawContract;
  let contract;
  let validateSTPacket;
  let validateSTPacketContractsMock;
  let validateSTPacketDocumentsMock;

  beforeEach(function beforeEach() {
    contract = getContractFixture();
    rawContract = contract.toJSON();

    stPacket = getSTPacketFixture();
    rawSTPacket = stPacket.toJSON();

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateSTPacketContractsMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validateSTPacketDocumentsMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketContractsMock,
      validateSTPacketDocumentsMock,
    );
  });

  describe('contractId', () => {
    it('should be present', () => {
      delete rawSTPacket.contractId;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('contractId');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.contractId = 1;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no less than 32 chars', () => {
      rawSTPacket.contractId = '86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no longer than 44 chars', () => {
      rawSTPacket.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });
  });

  describe('itemsMerkleRoot', () => {
    it('should be present', () => {
      delete rawSTPacket.itemsMerkleRoot;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsMerkleRoot');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsMerkleRoot = 1;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no less than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no longer than 64 chars', () => {
      rawSTPacket.itemsMerkleRoot = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
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

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsHash');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be a string', () => {
      rawSTPacket.itemsHash = 1;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no less than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('minLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be no longer than 64 chars', () => {
      rawSTPacket.itemsHash = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('maxLength');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
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
      delete rawSTPacket.documents;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('documents');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be an array', () => {
      rawSTPacket.documents = 1;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should contain no more than 1000 items', () => {
      const thousandDocuments = (new Array(1001)).fill(rawSTPacket.documents[0]);
      rawSTPacket.documents.push(...thousandDocuments);

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors).to.be.an('array').with.lengthOf(3);

      expect(errors[0].dataPath).to.equal('.documents');
      expect(errors[0].keyword).to.equal('maxItems');

      expect(errors[1].dataPath).to.equal('.documents');
      expect(errors[1].keyword).to.equal('maxItems');

      expect(errors[2].dataPath).to.equal('');
      expect(errors[2].keyword).to.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });
  });

  describe('contracts', () => {
    it('should be present', () => {
      delete rawSTPacket.contracts;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('contracts');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should be an array', () => {
      rawSTPacket.contracts = 1;

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contracts');
      expect(error.keyword).to.equal('type');

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });

    it('should contain no more than one contract', () => {
      rawSTPacket.contracts.push(rawContract, rawContract);

      const result = validateSTPacket(rawSTPacket, contract);

      expectJsonSchemaError(result, 3);

      const errors = result.getErrors();

      expect(errors[0].dataPath).to.equal('.documents');
      expect(errors[0].keyword).to.equal('maxItems');

      expect(errors[1].dataPath).to.equal('.contracts');
      expect(errors[1].keyword).to.equal('maxItems');

      expect(errors[2].dataPath).to.equal('');
      expect(errors[2].keyword).to.equal('oneOf');
      expect(errors[2].params.passingSchemas).to.be.null();

      expect(validateSTPacketContractsMock).to.have.not.been.called();
      expect(validateSTPacketDocumentsMock).to.have.not.been.called();
    });
  });

  it('should return invalid result if packet is empty', () => {
    rawSTPacket.contracts = [];
    rawSTPacket.documents = [];

    const result = validateSTPacket(rawSTPacket, contract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.keyword).to.equal('oneOf');
    expect(error.params.passingSchemas).to.deep.equal([0, 1]);

    expect(validateSTPacketContractsMock).to.have.not.been.called();
    expect(validateSTPacketDocumentsMock).to.have.not.been.called();
  });

  it('should return invalid result if packet contains the both documents and contracts', () => {
    rawSTPacket.contracts.push(rawContract);

    const result = validateSTPacket(rawSTPacket, contract);

    expectJsonSchemaError(result, 3);

    const errors = result.getErrors();

    expect(errors[0].dataPath).to.equal('.documents');
    expect(errors[0].keyword).to.equal('maxItems');

    expect(errors[1].dataPath).to.equal('.contracts');
    expect(errors[1].keyword).to.equal('maxItems');

    expect(errors[2].dataPath).to.equal('');
    expect(errors[2].keyword).to.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();

    expect(validateSTPacketContractsMock).to.have.not.been.called();
    expect(validateSTPacketDocumentsMock).to.have.not.been.called();
  });

  it('should return invalid result if there are additional properties in the packet', () => {
    const additionalProperty = 'additionalStuff';

    rawSTPacket[additionalProperty] = {};

    const result = validateSTPacket(rawSTPacket, contract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('additionalProperties');
    expect(error.params.additionalProperty).to.equal(additionalProperty);

    expect(validateSTPacketContractsMock).to.have.not.been.called();
    expect(validateSTPacketDocumentsMock).to.have.not.been.called();
  });

  it('should validate Contract if present', () => {
    stPacket.setDocuments([]);
    stPacket.setContract(contract);

    rawSTPacket = stPacket.toJSON();

    const contractError = new ConsensusError('test');

    validateSTPacketContractsMock.returns(
      new ValidationResult([contractError]),
    );

    const result = validateSTPacket(rawSTPacket);

    expectValidationError(result);

    expect(validateSTPacketContractsMock).to.have.been.calledOnceWith(rawSTPacket);

    const [error] = result.getErrors();

    expect(error).to.equal(contractError);
  });

  it('should validate Documents if present', () => {
    const contractError = new ConsensusError('test');

    validateSTPacketDocumentsMock.returns(
      new ValidationResult([contractError]),
    );

    const result = validateSTPacket(rawSTPacket, contract);

    expectValidationError(result);

    expect(validateSTPacketDocumentsMock).to.have.been.calledOnceWith(
      rawSTPacket,
      contract,
    );

    const [error] = result.getErrors();

    expect(error).to.equal(contractError);
  });

  it('should return valid result if packet structure is correct', () => {
    const result = validateSTPacket(rawSTPacket);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
