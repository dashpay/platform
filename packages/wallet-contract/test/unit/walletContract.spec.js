const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const walletContractDocumentsSchema = require('../../schema/v1/wallet-contract-documents.json');

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

describe('Wallet Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    dataContract = dpp.dataContract.create(identityId, BigInt(1), walletContractDocumentsSchema);
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), walletContractDocumentsSchema))
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
              .equal('maxItems');
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

    describe('verifyIdentity', () => {
      let rawIdentityVerifyDocument;

      beforeEach(async () => {
        rawIdentityVerifyDocument = {
          normalizedLabel: 'wa11et', // lower case and base58 chars only
          normalizedParentDomainName: 'dash',
          url: "https://dash.org"
        };
      });

      describe('normalizedLabel', () => {
        it('should be defined', async () => {
          delete rawIdentityVerifyDocument.normalizedLabel;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('normalizedLabel');
        });

        it('should follow pattern', async () => {
          rawIdentityVerifyDocument.normalizedLabel = 'InValiD label';

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/normalizedLabel');
        });

        it('should be less than 63 chars', async () => {
          rawIdentityVerifyDocument.normalizedLabel = 'a'.repeat(64);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/normalizedLabel');
        });
      });

      describe('normalizedParentDomainName', () => {
        it('should be defined', async () => {
          delete rawIdentityVerifyDocument.normalizedParentDomainName;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('normalizedParentDomainName');
        });

        it('should be less than 190 chars', async () => {
          rawIdentityVerifyDocument.normalizedParentDomainName = 'a'.repeat(191);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/normalizedParentDomainName');
        });

        it('should follow pattern', async () => {
          rawIdentityVerifyDocument.normalizedParentDomainName = '&'.repeat(50);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/normalizedParentDomainName');
        });
      });

      describe('url', () => {
        it('should be defined', async () => {
          delete rawIdentityVerifyDocument.url;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
              .to
              .equal('required');
          expect(error.params.missingProperty)
              .to
              .equal('url');
        });

        it('should be less than 128 chars', async () => {
          rawIdentityVerifyDocument.normalizedParentDomainName = 'a'.repeat(129);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
              .to
              .equal('pattern');
          expect(error.instancePath)
              .to
              .equal('/url');
        });

        it('should follow pattern', async () => {
          rawIdentityVerifyDocument.url = '&'.repeat(50);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
              .to
              .equal('pattern');
          expect(error.instancePath)
              .to
              .equal('/url');
        });
      });

      it('should be valid', async () => {
        const domain = dpp.document.create(dataContract, identityId, 'domain', rawIdentityVerifyDocument);

        const result = await domain.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });
    });
  });
});
