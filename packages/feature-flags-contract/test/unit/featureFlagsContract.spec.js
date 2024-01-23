const { DashPlatformProtocol, JsonSchemaError } = require('@dashevo/wasm-dpp');

const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const crypto = require('crypto');
const featureFlagsContractDocumentsSchema = require('../../schema/v1/feature-flags-documents.json');

const expectJsonSchemaError = (validationResult, errorCount = 1) => {
  const errors = validationResult.getErrors();
  expect(errors).to.have.length(errorCount);

  const error = validationResult.getErrors()[0];
  expect(error).to.be.instanceof(JsonSchemaError);

  return error;
};

describe('Feature Flags contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    dataContract = dpp.dataContract.create(identityId, featureFlagsContractDocumentsSchema);
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, featureFlagsContractDocumentsSchema))
      .to.not.throw();
  });

  describe('documents', () => {
    describe('updateConsensusParams', () => {
      let rawUpdateConsensusParamsDocument;

      beforeEach(() => {
        rawUpdateConsensusParamsDocument = {
          enableAtHeight: 42,
          block: {
            maxBytes: 42,
            maxGas: 3,
          },
          version: {
            appVersion: 1,
          },
          evidence: {
            maxAgeNumBlocks: 1,
            maxAgeDuration: {
              seconds: 1,
              nanos: 0,
            },
            maxBytes: 1,
          },
        };
      });

      it('should have at least three properties', () => {
        rawUpdateConsensusParamsDocument = {};

        const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult, 2);

        expect(error.keyword).to.equal('minProperties');
        expect(error.params.minProperties).to.equal(3);
      });

      it('should not have additional properties', async () => {
        rawUpdateConsensusParamsDocument = {
          ...rawUpdateConsensusParamsDocument,
          someOtherProperty: 42,
          block: {
            maxBytes: 1,
            maxGas: 1,
          },
        };

        const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
        const validationResult = document.validate(dpp.protocolVersion);
        const error = expectJsonSchemaError(validationResult);

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['someOtherProperty']);
      });

      describe('enabledAtHeight', () => {
        it('should be present', async () => {
          delete rawUpdateConsensusParamsDocument.enableAtHeight;

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('enableAtHeight');
        });

        it('should be integer', () => {
          rawUpdateConsensusParamsDocument.enableAtHeight = 'string';

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('type');
          expect(error.params.type).to.equal('integer');
        });

        it('should be at least 1', () => {
          rawUpdateConsensusParamsDocument.enableAtHeight = 0;

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minimum');
          expect(error.params.minimum).to.equal(1);
        });
      });

      describe('block', () => {
        beforeEach(() => {
          rawUpdateConsensusParamsDocument.block = {
            maxBytes: 42,
            maxGas: 42,
          };
        });

        it('should have at least on property', async () => {
          rawUpdateConsensusParamsDocument.block = {};

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minProperties');
          expect(error.params.minProperties).to.equal(1);
        });

        describe('maxBytes', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxBytes: 'string',
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          });

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxBytes: 0,
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('minimum');
            expect(error.params.minimum).to.equal(1);
          });
        });

        describe('maxGas', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxGas: 'string',
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          });

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxGas: 0,
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('minimum');
            expect(error.params.minimum).to.equal(1);
          });
        });

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = updateConsensusParams.validate(dpp.protocolVersion);

          expect(result.isValid()).to.be.true();
        });
      });

      describe('evidence', () => {
        beforeEach(() => {
          rawUpdateConsensusParamsDocument.evidence = {
            maxAgeNumBlocks: 42,
            maxBytes: 42,
            maxAgeDuration: {
              seconds: 42,
              nanos: 42,
            },
          };
        });

        it('should have at least on property', async () => {
          rawUpdateConsensusParamsDocument.evidence = {};

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minProperties');
          expect(error.params.minProperties).to.equal(1);
        });

        describe('maxBytes', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxBytes: 'string',
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          });

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxBytes: 0,
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('minimum');
            expect(error.params.minimum).to.equal(1);
          });
        });

        describe('maxAgeNumBlocks', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxAgeNumBlocks: 'string',
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          });

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxAgeNumBlocks: 0,
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('minimum');
            expect(error.params.minimum).to.equal(1);
          });
        });

        describe('maxAgeDuration', () => {
          describe('seconds', () => {
            it('should be present', async () => {
              delete rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds;

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('required');
              expect(error.params.missingProperty).to.equal('seconds');
            });

            it('should be integer', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds = 'string';

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('type');
              expect(error.params.type).to.equal('integer');
            });

            it('should be at least 1', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds = 0;

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('minimum');
              expect(error.params.minimum).to.equal(1);
            });
          });

          describe('nanos', () => {
            it('should be present', async () => {
              delete rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos;

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('required');
              expect(error.params.missingProperty).to.equal('nanos');
            });

            it('should be integer', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos = 'string';

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('type');
              expect(error.params.type).to.equal('integer');
            });

            it('should be at least 0', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos = -1;

              const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
              const validationResult = document.validate(dpp.protocolVersion);
              const error = expectJsonSchemaError(validationResult);

              expect(error.keyword).to.equal('minimum');
              expect(error.params.minimum).to.equal(0);
            });
          });
        });

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = updateConsensusParams.validate(dpp.protocolVersion);

          expect(result.isValid()).to.be.true();
        });
      });

      describe('version', () => {
        beforeEach(() => {
          rawUpdateConsensusParamsDocument.version = {
            appVersion: 42,
          };
        });

        it('should have at least on property', async () => {
          rawUpdateConsensusParamsDocument.version = {};

          const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword).to.equal('minProperties');
          expect(error.params.minProperties).to.equal(1);
        });

        describe('appVersion', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.version = {
              appVersion: 'string',
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('integer');
          });

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.version = {
              appVersion: 0,
            };

            const document = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);
            const validationResult = document.validate(dpp.protocolVersion);
            const error = expectJsonSchemaError(validationResult);

            expect(error.keyword).to.equal('minimum');
            expect(error.params.minimum).to.equal(1);
          });
        });

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = updateConsensusParams.validate(dpp.protocolVersion);

          expect(result.isValid()).to.be.true();
        });
      });

      it('should be valid', async () => {
        const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

        const result = updateConsensusParams.validate(dpp.protocolVersion);

        expect(result.isValid()).to.be.true();
      });
    });
  });
});
