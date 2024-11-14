const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const dpnsContractDocumentsSchema = require('../../schema/v1/dpns-contract-documents.json');

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

describe('DPNS Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    dataContract = dpp.dataContract.create(identityId, BigInt(1), dpnsContractDocumentsSchema);
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), dpnsContractDocumentsSchema))
      .to
      .not
      .throw();
  });

  describe('documents', () => {
    describe('preorder', () => {
      let rawPreorderDocument;

      beforeEach(() => {
        rawPreorderDocument = {
          saltedDomainHash: crypto.randomBytes(32),
        };
      });

      describe('saltedDomainHash', () => {
        it('should be defined', async () => {
          delete rawPreorderDocument.saltedDomainHash;

          const document = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('saltedDomainHash');
        });

        it('should not be empty', async () => {
          rawPreorderDocument.saltedDomainHash = Buffer.alloc(0);

          const document = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/saltedDomainHash');
        });

        it('should be not less than 32 bytes', async () => {
          rawPreorderDocument.saltedDomainHash = crypto.randomBytes(10);

          const document = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .equal('/saltedDomainHash');
        });

        it('should be not longer than 32 bytes', async () => {
          rawPreorderDocument.saltedDomainHash = crypto.randomBytes(40);

          const document = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .equal('/saltedDomainHash');
        });
      });

      it('should not have additional properties', async () => {
        rawPreorderDocument.someOtherProperty = 42;

        const document = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);
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
        const preorder = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

        const result = await preorder.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });
    });

    describe('domain', () => {
      let rawDomainDocument;

      beforeEach(async () => {
        rawDomainDocument = {
          label: 'Wallet',
          normalizedLabel: 'wa11et', // lower case and base58 chars only
          normalizedParentDomainName: 'dash',
          preorderSalt: crypto.randomBytes(32),
          records: {
            identity: await generateRandomIdentifier(),
          },
          subdomainRules: {
            allowSubdomains: false,
          },
        };
      });

      describe('label', () => {
        it('should be present', async () => {
          delete rawDomainDocument.label;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('label');
        });

        it('should follow pattern', async () => {
          rawDomainDocument.label = 'invalid label';

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/label');
        });

        it('should be longer than 3 chars', async () => {
          rawDomainDocument.label = 'ab';

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minLength');
          expect(error.instancePath)
            .to
            .equal('/label');
        });

        it('should be less than 63 chars', async () => {
          rawDomainDocument.label = 'a'.repeat(64);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('pattern');
          expect(error.instancePath)
            .to
            .equal('/label');
        });
      });

      describe('normalizedLabel', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.normalizedLabel;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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
          rawDomainDocument.normalizedLabel = 'InValiD label';

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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
          rawDomainDocument.normalizedLabel = 'a'.repeat(64);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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
          delete rawDomainDocument.normalizedParentDomainName;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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
          rawDomainDocument.normalizedParentDomainName = 'a'.repeat(191);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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
          rawDomainDocument.normalizedParentDomainName = '&'.repeat(50);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
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

      describe('preorderSalt', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.preorderSalt;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('preorderSalt');
        });

        it('should not be empty', async () => {
          rawDomainDocument.preorderSalt = Buffer.alloc(0);

          dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/preorderSalt');
        });

        it('should be not less than 32 bytes', async () => {
          rawDomainDocument.preorderSalt = crypto.randomBytes(10);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .equal('/preorderSalt');
        });

        it('should be not longer than 32 bytes', async () => {
          rawDomainDocument.preorderSalt = crypto.randomBytes(40);

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .equal('/preorderSalt');
        });
      });

      it('should not have additional properties', async () => {
        rawDomainDocument.someOtherProperty = [];

        const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

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
        const domain = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

        const result = await domain.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });

      describe('Records', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.records;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('records');
        });

        it('should not be empty', async () => {
          rawDomainDocument.records = {};

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minProperties');
          expect(error.instancePath)
            .to
            .deep
            .equal('/records');
        });

        it('should not have additional properties', async () => {
          rawDomainDocument.records = {
            someOtherProperty: 42,
          };

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('additionalProperties');
          expect(error.instancePath)
            .to
            .equal('/records');
          expect(error.params.additionalProperties)
            .to
            .deep
            .equal(['someOtherProperty']);
        });

        describe('Dash Identity', () => {
          describe('identity record', () => {
            it('should no less than 32 bytes', async () => {
              rawDomainDocument.records = {
                identity: crypto.randomBytes(30),
              };

              expect(() => {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
              })
                .to
                .throw();
            });

            it('should no more than 32 bytes', async () => {
              rawDomainDocument.records = {
                identity: crypto.randomBytes(64),
              };

              expect(() => {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
              })
                .to
                .throw();
            });
          });
        });
      });

      describe('subdomainRules', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.subdomainRules;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('subdomainRules');
        });

        it('should not have additional properties', async () => {
          rawDomainDocument.subdomainRules.someOtherProperty = 42;

          const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('additionalProperties');
          expect(error.instancePath)
            .to
            .equal('/subdomainRules');
          expect(error.params.additionalProperties)
            .to
            .deep
            .equal(['someOtherProperty']);
        });

        describe('allowSubdomains', () => {
          it('should be boolean', async () => {
            rawDomainDocument.subdomainRules.allowSubdomains = 'data';

            const document = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword)
              .to
              .equal('type');
            expect(error.instancePath)
              .to
              .deep
              .equal('/subdomainRules/allowSubdomains');
          });
        });
      });
    });
  });
});
