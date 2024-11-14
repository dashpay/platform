const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');

const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const crypto = require('crypto');
const withdrawalContractDocumentsSchema = require('../../schema/v1/withdrawals-documents.json');

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

describe('Withdrawals contract', () => {
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
      withdrawalContractDocumentsSchema,
    );
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), withdrawalContractDocumentsSchema))
      .to
      .not
      .throw();
  });

  describe('documents', () => {
    describe('withdrawal', () => {
      let rawWithdrawalDocument;

      beforeEach(() => {
        rawWithdrawalDocument = {
          transactionIndex: 42,
          amount: 1000,
          coreFeePerByte: 1,
          pooling: 0,
          outputScript: Buffer.alloc(23, 2),
          status: 0,
        };
      });

      describe('amount', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.amount;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('amount');
        });

        it('should be integer', () => {
          rawWithdrawalDocument.amount = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be at least 1000', () => {
          rawWithdrawalDocument.amount = 0;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.params.minimum)
            .to
            .equal(1000);
        });
      });

      describe('transactionSignHeight', () => {
        it('should be integer', () => {
          rawWithdrawalDocument.transactionSignHeight = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.transactionSignHeight = 0;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.params.minimum)
            .to
            .equal(1);
        });
      });

      describe('transactionIndex', () => {
        it('should be integer', () => {
          rawWithdrawalDocument.transactionIndex = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.transactionIndex = 0;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.params.minimum)
            .to
            .equal(1);
        });
      });

      describe('coreFeePerByte', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.coreFeePerByte;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('coreFeePerByte');
        });

        it('should be integer', () => {
          rawWithdrawalDocument.coreFeePerByte = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.coreFeePerByte = 0;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.params.minimum)
            .to
            .equal(1);
        });
      });

      describe('pooling', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.pooling;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('pooling');
        });

        it('should be integer', () => {
          rawWithdrawalDocument.pooling = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be within enum range', () => {
          rawWithdrawalDocument.pooling = 3;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('enum');
        });
      });

      describe('outputScript', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.outputScript;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('outputScript');
        });

        it('should be byte array', () => {
          rawWithdrawalDocument.outputScript = 1;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('array');
        });

        it('should be not less then 23 bytes long', () => {
          rawWithdrawalDocument.outputScript = [0];

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.params.minItems)
            .to
            .equal(23);
        });

        it('should be not more then 25 bytes long', () => {
          rawWithdrawalDocument.outputScript = Buffer.alloc(33);

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.params.maxItems)
            .to
            .equal(25);
        });
      });

      describe('status', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.status;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('status');
        });

        it('should be integer', () => {
          rawWithdrawalDocument.status = 'string';

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.params.type)
            .to
            .equal('integer');
        });

        it('should be within enum range', () => {
          rawWithdrawalDocument.status = 7;

          const document = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('enum');
        });
      });

      it('should be valid', async () => {
        const withdrawal = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

        const result = await withdrawal.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });
    });
  });
});
