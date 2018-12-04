const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

const validateSTPacketHeaderFactory = require('../../../../lib/stPacketHeader/validateSTPacketHeaderFactory');

const { expectJsonSchemaError } = require('../../../../lib/test/expect/expectError');

describe('validateSTPacketHeaderStructure', () => {
  let rawStPacketHeader;
  let validateSTPacketHeader;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateSTPacketHeader = validateSTPacketHeaderFactory(validator);

    rawStPacketHeader = {
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
    };
  });


  describe('contractId', () => {
    it('should be present', () => {
      delete rawStPacketHeader.contractId;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('contractId');
    });

    it('should be a string', () => {
      rawStPacketHeader.contractId = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64 chars', () => {
      rawStPacketHeader.contractId = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be longer than 64 chars', () => {
      rawStPacketHeader.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('maxLength');
    });
  });

  describe('itemsMerkleRoot', () => {
    it('should be present', () => {
      delete rawStPacketHeader.itemsMerkleRoot;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('itemsMerkleRoot');
    });

    it('should be a string', () => {
      rawStPacketHeader.itemsMerkleRoot = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64 chars', () => {
      rawStPacketHeader.itemsMerkleRoot = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be longer than 64 chars', () => {
      rawStPacketHeader.itemsMerkleRoot = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('maxLength');
    });
  });

  describe('itemsHash', () => {
    it('should be present', () => {
      delete rawStPacketHeader.itemsHash;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('itemsHash');
    });

    it('should be a string', () => {
      rawStPacketHeader.itemsHash = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64 chars', () => {
      rawStPacketHeader.itemsHash = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be longer than 64 chars', () => {
      rawStPacketHeader.itemsHash = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('maxLength');
    });
  });

  it('should return valid result if packet structure is correct', () => {
    const result = validateSTPacketHeader(rawStPacketHeader);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
