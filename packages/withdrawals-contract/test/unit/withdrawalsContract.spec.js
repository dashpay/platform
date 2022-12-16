const DashPlatformProtocol = require('@dashevo/dpp');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const withdrawalContractDocumentsSchema = require('../../schema/withdrawals-documents.json');

describe('Withdrawals contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async function beforeEach() {
    const fetchContractStub = this.sinon.stub();

    dpp = new DashPlatformProtocol({
      stateRepository: {
        fetchDataContract: fetchContractStub,
      },
    });

    await dpp.initialize();

    identityId = generateRandomIdentifier();

    dataContract = dpp.dataContract.create(identityId, withdrawalContractDocumentsSchema);

    fetchContractStub.resolves(dataContract);
  });

  it('should have a valid contract definition', async function shouldHaveValidContract() {
    this.timeout(5000);

    const validationResult = await dpp.dataContract.validate(dataContract);

    expect(validationResult.isValid()).to.be.true();
  });

  describe('documents', () => {
    describe('withdrawal', () => {
      let rawWithdrawalDocument;

      beforeEach(() => {
        rawWithdrawalDocument = {
          transactionId: Buffer.alloc(32, 1),
          transactionIndex: 42,
          amount: 1000,
          coreFeePerByte: 1,
          pooling: 0,
          outputScript: Buffer.alloc(23, 2),
          status: 0,
        };
      });

      it('should have at least five properties', () => {
        rawWithdrawalDocument = {
          $createdAt: (new Date()).getTime(),
          $updatedAt: (new Date()).getTime(),
        };

        try {
          dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

          expect.fail('should throw error');
        } catch (e) {
          expect(e.name).to.equal('InvalidDocumentError');
          expect(e.getErrors()).to.have.a.lengthOf(1);

          const [error] = e.getErrors();

          expect(error.name).to.equal('JsonSchemaError');
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('amount');
        }
      });

      it('should not have additional properties', async () => {
        rawWithdrawalDocument.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

          expect.fail('should throw error');
        } catch (e) {
          expect(e.name).to.equal('InvalidDocumentError');
          expect(e.getErrors()).to.have.a.lengthOf(1);

          const [error] = e.getErrors();

          expect(error.name).to.equal('JsonSchemaError');
          expect(error.keyword).to.equal('additionalProperties');
          expect(error.params.additionalProperty).to.equal('someOtherProperty');
        }
      });

      describe('transactionId', () => {
        it('should be byte array', () => {
          rawWithdrawalDocument.transactionId = 1;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('array');
          }
        });

        it('should be not less then 32 bytes long', () => {
          rawWithdrawalDocument.transactionId = [0];

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minItems');
            expect(error.params.limit).to.equal(32);
          }
        });

        it('should be not more then 32 bytes long', () => {
          rawWithdrawalDocument.transactionId = Buffer.alloc(33);

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxItems');
            expect(error.params.limit).to.equal(32);
          }
        });
      });

      describe('amount', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.amount;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('amount');
          }
        });

        it('should be integer', () => {
          rawWithdrawalDocument.amount = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be at least 1000', () => {
          rawWithdrawalDocument.amount = 0;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minimum');
            expect(error.params.limit).to.equal(1000);
          }
        });
      });

      describe('transactionSignHeight', () => {
        it('should be integer', () => {
          rawWithdrawalDocument.transactionSignHeight = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.transactionSignHeight = 0;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minimum');
            expect(error.params.limit).to.equal(1);
          }
        });
      });

      describe('transactionIndex', () => {
        it('should be integer', () => {
          rawWithdrawalDocument.transactionIndex = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.transactionIndex = 0;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minimum');
            expect(error.params.limit).to.equal(1);
          }
        });
      });

      describe('coreFeePerByte', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.coreFeePerByte;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('coreFeePerByte');
          }
        });

        it('should be integer', () => {
          rawWithdrawalDocument.coreFeePerByte = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be at least 1', () => {
          rawWithdrawalDocument.coreFeePerByte = 0;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minimum');
            expect(error.params.limit).to.equal(1);
          }
        });
      });

      describe('pooling', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.pooling;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('pooling');
          }
        });

        it('should be integer', () => {
          rawWithdrawalDocument.pooling = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be within enum range', () => {
          rawWithdrawalDocument.pooling = 1;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('enum');
          }
        });
      });

      describe('outputScript', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.outputScript;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('outputScript');
          }
        });

        it('should be byte array', () => {
          rawWithdrawalDocument.outputScript = 1;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('array');
          }
        });

        it('should be not less then 23 bytes long', () => {
          rawWithdrawalDocument.outputScript = [0];

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minItems');
            expect(error.params.limit).to.equal(23);
          }
        });

        it('should be not more then 25 bytes long', () => {
          rawWithdrawalDocument.outputScript = Buffer.alloc(33);

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxItems');
            expect(error.params.limit).to.equal(25);
          }
        });
      });

      describe('status', () => {
        it('should be present', async () => {
          delete rawWithdrawalDocument.status;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('status');
          }
        });

        it('should be integer', () => {
          rawWithdrawalDocument.status = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          }
        });

        it('should be within enum range', () => {
          rawWithdrawalDocument.status = 7;

          try {
            dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('enum');
          }
        });
      });

      it('should be valid', async () => {
        const withdrawal = dpp.document.create(dataContract, identityId, 'withdrawal', rawWithdrawalDocument);

        const result = await dpp.document.validate(withdrawal);

        expect(result.isValid()).to.be.true();
      });
    });
  });
});
