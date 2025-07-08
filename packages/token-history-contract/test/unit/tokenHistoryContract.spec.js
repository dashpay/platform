const crypto = require('crypto');
const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const { expect } = require('chai');
const tokenHistoryContractDocumentsSchema = require('../../schema/v1/token-history-contract-documents.json');

const expectJsonSchemaError = (validationResult, errorCount = 1) => {
  const errors = validationResult.getErrors();
  expect(errors).to.have.length(errorCount);

  const error = validationResult.getErrors()[0];
  expect(error).to.be.instanceof(JsonSchemaError);

  return error;
};

describe('Token History Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol({ generate: () => crypto.randomBytes(32) });
    identityId = await generateRandomIdentifier();
    dataContract = dpp.dataContract.create(
      identityId,
      BigInt(1),
      tokenHistoryContractDocumentsSchema,
    );
  });

  it('should have a valid contract definition', async () => {
    const createContract = () => dpp.dataContract.create(
      identityId,
      BigInt(1),
      tokenHistoryContractDocumentsSchema,
    );

    expect(createContract).to.not.throw();
  });

  describe('documents', () => {
    describe('burn', () => {
      let rawBurnDocument;

      beforeEach(() => {
        rawBurnDocument = {
          tokenId: crypto.randomBytes(32),
          burnFromId: crypto.randomBytes(32),
          amount: 100,
          note: 'Burning tokens',
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawBurnDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'burn', rawBurnDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      describe('amount', () => {
        it('should be defined', async () => {
          delete rawBurnDocument.amount;
          const document = dpp.document.create(dataContract, identityId, 'burn', rawBurnDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('amount');
        });

        it('should be a non-negative integer', async () => {
          rawBurnDocument.amount = -1;
          const document = dpp.document.create(dataContract, identityId, 'burn', rawBurnDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      it('should not have additional properties', async () => {
        rawBurnDocument.extraProp = 123;
        const document = dpp.document.create(dataContract, identityId, 'burn', rawBurnDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal([
          'extraProp',
        ]);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'burn', rawBurnDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('mint', () => {
      let rawMintDocument;

      beforeEach(() => {
        rawMintDocument = {
          tokenId: crypto.randomBytes(32),
          recipientId: crypto.randomBytes(32),
          amount: 1000,
          note: 'Minting tokens',
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawMintDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'mint', rawMintDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      describe('recipientId', () => {
        it('should be defined', async () => {
          delete rawMintDocument.recipientId;
          const document = dpp.document.create(dataContract, identityId, 'mint', rawMintDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('recipientId');
        });
      });

      describe('amount', () => {
        it('should be a non-negative integer', async () => {
          rawMintDocument.amount = -1;
          const document = dpp.document.create(dataContract, identityId, 'mint', rawMintDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      it('should not have additional properties', async () => {
        rawMintDocument.extraField = 'foo';
        const document = dpp.document.create(dataContract, identityId, 'mint', rawMintDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['extraField']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'mint', rawMintDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('transfer', () => {
      let rawTransferDocument;

      beforeEach(() => {
        rawTransferDocument = {
          tokenId: crypto.randomBytes(32),
          amount: 10,
          toIdentityId: crypto.randomBytes(32),
          publicNote: 'Transfer tokens',
          encryptedPersonalNote: crypto.randomBytes(32),
          encryptedSharedNote: crypto.randomBytes(32),
          senderKeyIndex: 0,
          recipientKeyIndex: 1,
          rootEncryptionKeyIndex: 2,
          derivationEncryptionKeyIndex: 3,
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawTransferDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      describe('amount', () => {
        it('should be a non-negative integer', async () => {
          rawTransferDocument.amount = -1;
          const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      describe('toIdentityId', () => {
        it('should be defined', async () => {
          delete rawTransferDocument.toIdentityId;
          const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('toIdentityId');
        });
      });

      it('should not have additional properties', async () => {
        rawTransferDocument.foo = 'bar';
        const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['foo']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('freeze', () => {
      let rawFreezeDocument;

      beforeEach(() => {
        rawFreezeDocument = {
          tokenId: crypto.randomBytes(32),
          frozenIdentityId: crypto.randomBytes(32),
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawFreezeDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'freeze', rawFreezeDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      describe('frozenIdentityId', () => {
        it('should be defined', async () => {
          delete rawFreezeDocument.frozenIdentityId;
          const document = dpp.document.create(dataContract, identityId, 'freeze', rawFreezeDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('frozenIdentityId');
        });
      });

      it('should not have additional properties', async () => {
        rawFreezeDocument.something = true;
        const document = dpp.document.create(dataContract, identityId, 'freeze', rawFreezeDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['something']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'freeze', rawFreezeDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('unfreeze', () => {
      let rawUnfreezeDocument;

      beforeEach(() => {
        rawUnfreezeDocument = {
          tokenId: crypto.randomBytes(32),
          frozenIdentityId: crypto.randomBytes(32),
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawUnfreezeDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'unfreeze', rawUnfreezeDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      it('should not have additional properties', async () => {
        rawUnfreezeDocument.foo = 'bar';
        const document = dpp.document.create(dataContract, identityId, 'unfreeze', rawUnfreezeDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['foo']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'unfreeze', rawUnfreezeDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('destroyFrozenFunds', () => {
      let rawDestroyFrozenFundsDocument;

      beforeEach(() => {
        rawDestroyFrozenFundsDocument = {
          tokenId: crypto.randomBytes(32),
          frozenIdentityId: crypto.randomBytes(32),
          destroyedAmount: 500,
        };
      });

      describe('frozenIdentityId', () => {
        it('should be defined', async () => {
          delete rawDestroyFrozenFundsDocument.frozenIdentityId;
          const document = dpp.document.create(dataContract, identityId, 'destroyFrozenFunds', rawDestroyFrozenFundsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('frozenIdentityId');
        });
      });

      describe('destroyedAmount', () => {
        it('should be non-negative', async () => {
          rawDestroyFrozenFundsDocument.destroyedAmount = -1;
          const document = dpp.document.create(dataContract, identityId, 'destroyFrozenFunds', rawDestroyFrozenFundsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      it('should not have additional properties', async () => {
        rawDestroyFrozenFundsDocument.bar = 123;
        const document = dpp.document.create(dataContract, identityId, 'destroyFrozenFunds', rawDestroyFrozenFundsDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['bar']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'destroyFrozenFunds', rawDestroyFrozenFundsDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });

    describe('emergencyAction', () => {
      let rawEmergencyActionDocument;

      beforeEach(() => {
        rawEmergencyActionDocument = {
          tokenId: crypto.randomBytes(32),
          action: 1,
        };
      });

      describe('tokenId', () => {
        it('should be defined', async () => {
          delete rawEmergencyActionDocument.tokenId;
          const document = dpp.document.create(dataContract, identityId, 'emergencyAction', rawEmergencyActionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('tokenId');
        });
      });

      describe('action', () => {
        it('should be non-negative', async () => {
          rawEmergencyActionDocument.action = -5;
          const document = dpp.document.create(dataContract, identityId, 'emergencyAction', rawEmergencyActionDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('enum');
        });
      });

      it('should not have additional properties', async () => {
        rawEmergencyActionDocument.xyz = 999;
        const document = dpp.document.create(dataContract, identityId, 'emergencyAction', rawEmergencyActionDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['xyz']);
      });

      it('should be valid', async () => {
        const document = dpp.document.create(dataContract, identityId, 'emergencyAction', rawEmergencyActionDocument);
        const validationResult = await document.validate(dpp.protocolVersion);
        expect(validationResult.isValid()).to.be.true();
      });
    });
  });
});
