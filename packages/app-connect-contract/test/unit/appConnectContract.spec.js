const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const appConnectContractDocumentsSchema = require('../../schema/v1/app-connect-contract-documents.json');

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

describe('App Connect Contract', () => {
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
      appConnectContractDocumentsSchema,
    );
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(
      identityId,
      BigInt(1),
      appConnectContractDocumentsSchema,
    ))
      .to
      .not
      .throw();
  });

  describe('documents', () => {
    describe('loginKeyResponse', () => {
      let rawLoginKeyResponseDocument;

      beforeEach(() => {
        rawLoginKeyResponseDocument = {
          appEphemeralPubKeyHash: crypto.randomBytes(20),
          walletEphemeralPubKey: crypto.randomBytes(33),
          encryptedPayload: crypto.randomBytes(60),
        };
      });

      describe('appEphemeralPubKeyHash', () => {
        it('should be defined', async () => {
          delete rawLoginKeyResponseDocument.appEphemeralPubKeyHash;

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('appEphemeralPubKeyHash');
        });

        it('should be not shorter than 20 bytes', async () => {
          rawLoginKeyResponseDocument.appEphemeralPubKeyHash = crypto.randomBytes(19);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minItems');
        });

        it('should be not longer than 20 bytes', async () => {
          rawLoginKeyResponseDocument.appEphemeralPubKeyHash = crypto.randomBytes(21);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxItems');
        });
      });

      describe('walletEphemeralPubKey', () => {
        it('should be defined', async () => {
          delete rawLoginKeyResponseDocument.walletEphemeralPubKey;

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('walletEphemeralPubKey');
        });

        it('should be not shorter than 33 bytes', async () => {
          rawLoginKeyResponseDocument.walletEphemeralPubKey = crypto.randomBytes(32);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minItems');
        });

        it('should be not longer than 33 bytes', async () => {
          rawLoginKeyResponseDocument.walletEphemeralPubKey = crypto.randomBytes(34);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxItems');
        });
      });

      describe('encryptedPayload', () => {
        it('should be defined', async () => {
          delete rawLoginKeyResponseDocument.encryptedPayload;

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('encryptedPayload');
        });

        it('should be not shorter than 60 bytes', async () => {
          rawLoginKeyResponseDocument.encryptedPayload = crypto.randomBytes(59);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minItems');
        });

        it('should be not longer than 572 bytes', async () => {
          rawLoginKeyResponseDocument.encryptedPayload = crypto.randomBytes(573);

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxItems');
        });

        it('should accept the maximum grant of seventeen keys', async () => {
          // Session key plus eight bindings asking for both
          // purposes: 28-byte envelope + 17 * 32 = 572 bytes.
          rawLoginKeyResponseDocument.encryptedPayload = crypto.randomBytes(28 + (17 * 32));

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });
      });

      it('should not have additional properties', async () => {
        rawLoginKeyResponseDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
        const validationResult = document.validate(dpp.protocolVersion);

        expect(validationResult.isValid()).to.be.true();
      });
    });
  });
});
