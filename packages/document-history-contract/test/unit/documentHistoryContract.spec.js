const crypto = require('crypto');
const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const { expect } = require('chai');
const documentHistoryContractDocumentsSchema = require('../../schema/v1/document-history-contract-documents.json');

const expectJsonSchemaError = (validationResult, errorCount = 1) => {
  const errors = validationResult.getErrors();
  expect(errors).to.have.length(errorCount);

  const error = validationResult.getErrors()[0];
  expect(error).to.be.instanceof(JsonSchemaError);

  return error;
};

describe('Document History Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol({ generate: () => crypto.randomBytes(32) });
    identityId = await generateRandomIdentifier();
    dataContract = dpp.dataContract.create(
      identityId,
      BigInt(1),
      documentHistoryContractDocumentsSchema,
    );
  });

  it('should have a valid contract definition', async () => {
    const createContract = () => dpp.dataContract.create(
      identityId,
      BigInt(1),
      documentHistoryContractDocumentsSchema,
    );

    expect(createContract).to.not.throw();
  });

  describe('documents', () => {
    describe('transfer', () => {
      let rawTransferDocument;

      beforeEach(() => {
        rawTransferDocument = {
          dataContractId: crypto.randomBytes(32),
          documentTypeName: 'domain',
          documentId: crypto.randomBytes(32),
          toIdentityId: crypto.randomBytes(32),
        };
      });

      ['dataContractId', 'documentTypeName', 'documentId', 'toIdentityId'].forEach((property) => {
        describe(property, () => {
          it('should be defined', async () => {
            delete rawTransferDocument[property];
            const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal(property);
          });
        });
      });

      it('should not have additional properties', async () => {
        rawTransferDocument.someOtherProperty = 42;
        const document = dpp.document.create(dataContract, identityId, 'transfer', rawTransferDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });
    });

    describe('purchase', () => {
      let rawPurchaseDocument;

      beforeEach(() => {
        rawPurchaseDocument = {
          dataContractId: crypto.randomBytes(32),
          documentTypeName: 'domain',
          documentId: crypto.randomBytes(32),
          sellerId: crypto.randomBytes(32),
          price: 100000,
        };
      });

      ['dataContractId', 'documentTypeName', 'documentId', 'sellerId', 'price'].forEach((property) => {
        describe(property, () => {
          it('should be defined', async () => {
            delete rawPurchaseDocument[property];
            const document = dpp.document.create(dataContract, identityId, 'purchase', rawPurchaseDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal(property);
          });
        });
      });

      describe('price', () => {
        it('should be a non-negative integer', async () => {
          rawPurchaseDocument.price = -1;
          const document = dpp.document.create(dataContract, identityId, 'purchase', rawPurchaseDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);
          expect(error.keyword).to.equal('minimum');
        });
      });

      it('should not have additional properties', async () => {
        rawPurchaseDocument.someOtherProperty = 42;
        const document = dpp.document.create(dataContract, identityId, 'purchase', rawPurchaseDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });
    });

    describe('priceUpdate', () => {
      let rawPriceUpdateDocument;

      beforeEach(() => {
        rawPriceUpdateDocument = {
          dataContractId: crypto.randomBytes(32),
          documentTypeName: 'domain',
          documentId: crypto.randomBytes(32),
          price: 100000,
        };
      });

      ['dataContractId', 'documentTypeName', 'documentId', 'price'].forEach((property) => {
        describe(property, () => {
          it('should be defined', async () => {
            delete rawPriceUpdateDocument[property];
            const document = dpp.document.create(dataContract, identityId, 'priceUpdate', rawPriceUpdateDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal(property);
          });
        });
      });

      it('should not have additional properties', async () => {
        rawPriceUpdateDocument.someOtherProperty = 42;
        const document = dpp.document.create(dataContract, identityId, 'priceUpdate', rawPriceUpdateDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });
    });
  });
});
