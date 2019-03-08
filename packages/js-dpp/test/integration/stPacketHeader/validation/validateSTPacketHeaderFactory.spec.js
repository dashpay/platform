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

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('contractId');
    });

    it('should be a string', () => {
      rawStPacketHeader.contractId = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('type');
    });

    it('should be no less than 64 chars', () => {
      rawStPacketHeader.contractId = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be no longer than 64 chars', () => {
      rawStPacketHeader.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.contractId');
      expect(error.keyword).to.equal('maxLength');
    });
  });

  describe('itemsMerkleRoot', () => {
    it('should be present', () => {
      delete rawStPacketHeader.itemsMerkleRoot;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsMerkleRoot');
    });

    it('should be a string', () => {
      rawStPacketHeader.itemsMerkleRoot = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('type');
    });

    it('should be no less than 64 chars', () => {
      rawStPacketHeader.itemsMerkleRoot = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be no longer than 64 chars', () => {
      rawStPacketHeader.itemsMerkleRoot = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsMerkleRoot');
      expect(error.keyword).to.equal('maxLength');
    });
  });

  describe('itemsHash', () => {
    it('should be present', () => {
      delete rawStPacketHeader.itemsHash;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('itemsHash');
    });

    it('should be a string', () => {
      rawStPacketHeader.itemsHash = 1;

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('type');
    });

    it('should be no less than 64 chars', () => {
      rawStPacketHeader.itemsHash = '86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be no longer than 64 chars', () => {
      rawStPacketHeader.itemsHash = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const result = validateSTPacketHeader(rawStPacketHeader);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.itemsHash');
      expect(error.keyword).to.equal('maxLength');
    });
  });

  it('should return valid result if packet structure is correct', () => {
    const result = validateSTPacketHeader(rawStPacketHeader);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
