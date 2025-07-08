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

    dataContract = dpp.dataContract.create(identityId, BigInt(1), keywordSearchContractDocumentsSchema);
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), keywordSearchContractDocumentsSchema))
      .to
      .not
      .throw();
  });

  describe('documents', () => {
    describe('txMetadata', () => {
      let rawTxMetadataDocument;

      beforeEach(() => {
        rawTxMetadataDocument = {
          keyIndex: 0,
          encryptionKeyIndex: 100,
          encryptedMetadata: crypto.randomBytes(64),
        };
      });

      describe('keyIndex', () => {
        it('should be defined', async () => {
          delete rawTxMetadataDocument.keyIndex;

          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('keyIndex');
        });

        it('should be a non-negative integer', async () => {
          rawTxMetadataDocument.keyIndex = -1;
          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('encryptionKeyIndex', () => {
        it('should be defined', async () => {
          delete rawTxMetadataDocument.encryptionKeyIndex;

          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('encryptionKeyIndex');
        });

        it('should be a non-negative integer', async () => {
          rawTxMetadataDocument.encryptionKeyIndex = -1;
          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('encryptedMetadata', () => {
        it('should be defined', async () => {
          delete rawTxMetadataDocument.encryptedMetadata;

          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('encryptedMetadata');
        });

        it('should be not shorter than 32 bytes', async () => {
          rawTxMetadataDocument.encryptedMetadata = crypto.randomBytes(31);

          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .equal('/encryptedMetadata');
        });

        it('should be not longer than 4096 bytes', async () => {
          rawTxMetadataDocument.encryptedMetadata = crypto.randomBytes(4097);

          const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .equal('/encryptedMetadata');
        });
      });

      it('should not have additional properties', async () => {
        rawTxMetadataDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword)
          .to
          .equal('additionalProperties');
        expect(error.params.additionalProperties)
          .to
          .deep
          .equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const txMetadata = dpp.document.create(dataContract, identityId, 'txMetadata', rawTxMetadataDocument);

        const result = await txMetadata.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });
    });
  });
});
