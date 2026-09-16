const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const keywordSearchContractDocumentsSchema = require('../../schema/v1/keyword-search-contract-documents.json');

const expectJsonSchemaError = (validationResult, errorCount = 1) => {
  const errors = validationResult.getErrors();
  expect(errors)
    .to
    .have
    .length(errorCount);

  const error = validationResult.getErrors()[0];
  expect(error)
    .to
    .be
    .instanceof(JsonSchemaError);

  return error;
};

describe('Search Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    dataContract = dpp.dataContract.create(
      identityId,
      BigInt(1),
      keywordSearchContractDocumentsSchema,
    );
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(
      identityId,
      BigInt(1),
      keywordSearchContractDocumentsSchema,
    ))
      .to
      .not
      .throw();
  });

  describe('documents', () => {
    describe('contractKeywords', () => {
      let rawContractKeywordsDocument;

      beforeEach(() => {
        rawContractKeywordsDocument = {
          keyword: 'accounting',
          contractId: crypto.randomBytes(32),
        };
      });

      describe('keyword', () => {
        it('should be defined', async () => {
          delete rawContractKeywordsDocument.keyword;

          const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('keyword');
        });

        it('should be not shorter than 3 characters', async () => {
          rawContractKeywordsDocument.keyword = 'ab';

          const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minLength');
        });

        it('should be not longer than 50 characters', async () => {
          rawContractKeywordsDocument.keyword = 'a'.repeat(51);

          const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxLength');
        });
      });

      describe('contractId', () => {
        it('should be defined', async () => {
          delete rawContractKeywordsDocument.contractId;

          const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('contractId');
        });

        it('should be exactly 32 bytes long', async () => {
          rawContractKeywordsDocument.contractId = crypto.randomBytes(31);

          // Identifier-typed byte arrays are converted at document creation,
          // so a wrong-length value throws there instead of surfacing as a
          // JSON-schema validation error.
          let error;
          try {
            dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
          } catch (e) {
            error = e;
          }

          expect(error).to.exist();
          expect(String(error)).to.contain('not 32 bytes long');
        });
      });

      it('should not have additional properties', async () => {
        rawContractKeywordsDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'contractKeywords', rawContractKeywordsDocument);
        const validationResult = document.validate(dpp.protocolVersion);

        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('shortDescription', () => {
      let rawShortDescriptionDocument;

      beforeEach(() => {
        rawShortDescriptionDocument = {
          contractId: crypto.randomBytes(32),
          description: 'A short description of the contract',
        };
      });

      describe('description', () => {
        it('should be defined', async () => {
          delete rawShortDescriptionDocument.description;

          const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('description');
        });

        it('should be not shorter than 3 characters', async () => {
          rawShortDescriptionDocument.description = 'ab';

          const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minLength');
        });

        it('should be not longer than 100 characters', async () => {
          rawShortDescriptionDocument.description = 'a'.repeat(101);

          const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxLength');
        });
      });

      describe('contractId', () => {
        it('should be defined', async () => {
          delete rawShortDescriptionDocument.contractId;

          const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('contractId');
        });
      });

      it('should not have additional properties', async () => {
        rawShortDescriptionDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'shortDescription', rawShortDescriptionDocument);
        const validationResult = document.validate(dpp.protocolVersion);

        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('fullDescription', () => {
      let rawFullDescriptionDocument;

      beforeEach(() => {
        rawFullDescriptionDocument = {
          contractId: crypto.randomBytes(32),
          description: 'A much longer description of the contract and everything it does',
        };
      });

      describe('description', () => {
        it('should be defined', async () => {
          delete rawFullDescriptionDocument.description;

          const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('description');
        });

        it('should be not shorter than 3 characters', async () => {
          rawFullDescriptionDocument.description = 'ab';

          const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minLength');
        });

        // The schema's maxLength (10000) sits above the system per-field size
        // cap (5120), so an overlong description surfaces as a field-size
        // error rather than a JSON-schema maxLength error.
        it('should be rejected when longer than the system maximum field size', async () => {
          rawFullDescriptionDocument.description = 'a'.repeat(10001);

          const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.false();
          const [error] = validationResult.getErrors();
          expect(error.message).to.contain('more than system maximum');
        });
      });

      describe('contractId', () => {
        it('should be defined', async () => {
          delete rawFullDescriptionDocument.contractId;

          const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('contractId');
        });
      });

      it('should not have additional properties', async () => {
        rawFullDescriptionDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'fullDescription', rawFullDescriptionDocument);
        const validationResult = document.validate(dpp.protocolVersion);

        expect(validationResult.isValid()).to.be.true();
      });
    });
  });
});
