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
          contractId: crypto.randomBytes(32),
          appEphemeralPubKeyHash: crypto.randomBytes(20),
          walletEphemeralPubKey: crypto.randomBytes(33),
          encryptedPayload: crypto.randomBytes(60),
        };
      });

      describe('contractId', () => {
        it('should be defined', async () => {
          delete rawLoginKeyResponseDocument.contractId;

          const document = dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('contractId');
        });

        it('should be exactly 32 bytes long', async () => {
          rawLoginKeyResponseDocument.contractId = crypto.randomBytes(31);

          // Identifier-typed byte arrays are converted at document creation,
          // so a wrong-length value throws there instead of surfacing as a
          // JSON-schema validation error.
          let error;
          try {
            dpp.document.create(dataContract, identityId, 'loginKeyResponse', rawLoginKeyResponseDocument);
          } catch (e) {
            error = e;
          }

          expect(error).to.exist();
          expect(String(error)).to.contain('not 32 bytes long');
        });
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
          // Session key plus eight requestedEncryptionKeys records asking for both
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

    describe('appManifest', () => {
      let rawAppManifestDocument;

      beforeEach(() => {
        rawAppManifestDocument = {
          appContractId: crypto.randomBytes(32),
          name: 'Yappr',
          url: 'https://yap.pr',
          authBoundsKind: 3,
          authBoundsId: crypto.randomBytes(32),
          sessionSeconds: 604800,
          sessionBudget: 10000000000,
          requestedEncryptionKeys: crypto.randomBytes(96 * 4),
        };
      });

      describe('appContractId', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.appContractId;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('appContractId');
        });

        it('should be exactly 32 bytes long', async () => {
          rawAppManifestDocument.appContractId = crypto.randomBytes(33);

          let error;
          try {
            dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          } catch (e) {
            error = e;
          }

          expect(error).to.exist();
          expect(String(error)).to.contain('not 32 bytes long');
        });
      });

      describe('name', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.name;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('name');
        });

        it('should be not longer than 64 characters', async () => {
          rawAppManifestDocument.name = 'a'.repeat(65);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxLength');
        });
      });

      describe('url', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.url;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('url');
        });

        it('should be not longer than 256 characters', async () => {
          rawAppManifestDocument.url = 'a'.repeat(257);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxLength');
        });
      });

      describe('authBoundsKind', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.authBoundsKind;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('authBoundsKind');
        });

        it('should be not greater than 3', async () => {
          rawAppManifestDocument.authBoundsKind = 4;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maximum');
        });

        it('should be not less than 0', async () => {
          rawAppManifestDocument.authBoundsKind = -1;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('authBoundsId', () => {
        it('should be optional', async () => {
          delete rawAppManifestDocument.authBoundsId;
          rawAppManifestDocument.authBoundsKind = 0;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });

        it('should be not shorter than 32 bytes', async () => {
          rawAppManifestDocument.authBoundsId = crypto.randomBytes(31);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minItems');
        });

        it('should be not longer than 32 bytes', async () => {
          rawAppManifestDocument.authBoundsId = crypto.randomBytes(33);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxItems');
        });
      });

      describe('authBoundsDocType', () => {
        it('should be optional', async () => {
          rawAppManifestDocument.authBoundsKind = 2;
          rawAppManifestDocument.authBoundsDocType = 'post';

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });

        it('should be not longer than 64 characters', async () => {
          rawAppManifestDocument.authBoundsDocType = 'a'.repeat(65);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxLength');
        });
      });

      describe('sessionSeconds', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.sessionSeconds;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('sessionSeconds');
        });

        it('should be not less than 0', async () => {
          rawAppManifestDocument.sessionSeconds = -1;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('sessionBudget', () => {
        it('should be defined', async () => {
          delete rawAppManifestDocument.sessionBudget;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('sessionBudget');
        });

        it('should be not less than 0', async () => {
          rawAppManifestDocument.sessionBudget = -1;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('requestedEncryptionKeys', () => {
        it('should be optional', async () => {
          delete rawAppManifestDocument.requestedEncryptionKeys;

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });

        it('should accept an empty array', async () => {
          rawAppManifestDocument.requestedEncryptionKeys = Buffer.alloc(0);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });

        it('should accept eight records', async () => {
          rawAppManifestDocument.requestedEncryptionKeys = crypto.randomBytes(96 * 8);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);

          expect(validationResult.isValid()).to.be.true();
        });

        it('should be not longer than 768 bytes', async () => {
          rawAppManifestDocument.requestedEncryptionKeys = crypto.randomBytes(769);

          const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('maxItems');
        });
      });

      it('should not have additional properties', async () => {
        rawAppManifestDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'appManifest', rawAppManifestDocument);
        const validationResult = document.validate(dpp.protocolVersion);

        expect(validationResult.isValid()).to.be.true();
      });
    });
  });
});
