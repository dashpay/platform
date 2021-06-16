const DashPlatformProtocol = require('@dashevo/dpp');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const featureFlagsContractDocumentsSchema = require('../../schema/feature-flags-documents.json');

describe('Feature Flags contract', () => {
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

    dataContract = dpp.dataContract.create(identityId, featureFlagsContractDocumentsSchema);

    fetchContractStub.resolves(dataContract);
  });

  it('should have a valid contract definition', async () => {
    const validationResult = await dpp.dataContract.validate(dataContract);

    expect(validationResult.isValid()).to.be.true();
  });

  describe('documents', () => {
    describe('updateConsensusParams', () => {
      let rawUpdateConsensusParamsDocument;

      beforeEach(() => {
        rawUpdateConsensusParamsDocument = {
          enableAtHeight: 42,
        };
      });

      it('should have at least three properties', () => {
        rawUpdateConsensusParamsDocument = {
          $createdAt: (new Date()).getTime(),
          $updatedAt: (new Date()).getTime(),
        };

        try {
          dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          expect.fail('should throw error');
        } catch (e) {
          expect(e.name).to.equal('InvalidDocumentError');
          expect(e.getErrors()).to.have.a.lengthOf(1);

          const [error] = e.getErrors();

          expect(error.name).to.equal('JsonSchemaError');
          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal('enableAtHeight');
        }
      });

      it('should not have additional properties', async () => {
        rawUpdateConsensusParamsDocument.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

      describe('enabledAtHeight', () => {
        it('should be present', async () => {
          delete rawUpdateConsensusParamsDocument.enableAtHeight;

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('enableAtHeight');
          }
        });

        it('should be integer', () => {
          rawUpdateConsensusParamsDocument.enableAtHeight = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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
          rawUpdateConsensusParamsDocument.enableAtHeight = 0;

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

      describe('block', () => {
        beforeEach(() => {
          rawUpdateConsensusParamsDocument.block = {
            maxBytes: 42,
            maxGas: 42,
          };
        });

        it('should have at least on property', async () => {
          rawUpdateConsensusParamsDocument.block = {};

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minProperties');
            expect(error.params.limit).to.equal(1);
          }
        });

        describe('maxBytes', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxBytes: 'string',
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxBytes: 0,
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

        describe('maxGas', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxGas: 'string',
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.block = {
              maxGas: 0,
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = await dpp.document.validate(updateConsensusParams);

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

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minProperties');
            expect(error.params.limit).to.equal(1);
          }
        });

        describe('maxBytes', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxBytes: 'string',
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxBytes: 0,
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

        describe('maxAgeNumBlocks', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxAgeNumBlocks: 'string',
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.evidence = {
              maxAgeNumBlocks: 0,
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

        describe('maxAgeDuration', () => {
          describe('seconds', () => {
            it('should be present', async () => {
              delete rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds;

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('required');
                expect(error.params.missingProperty).to.equal('seconds');
              }
            });

            it('should be integer', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds = 'string';

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.seconds = 0;

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          describe('nanos', () => {
            it('should be present', async () => {
              delete rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos;

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('required');
                expect(error.params.missingProperty).to.equal('nanos');
              }
            });

            it('should be integer', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos = 'string';

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

            it('should be at least 0', () => {
              rawUpdateConsensusParamsDocument.evidence.maxAgeDuration.nanos = -1;

              try {
                dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('minimum');
                expect(error.params.limit).to.equal(0);
              }
            });
          });
        });

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = await dpp.document.validate(updateConsensusParams);

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

          try {
            dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minProperties');
            expect(error.params.limit).to.equal(1);
          }
        });

        describe('appVersion', () => {
          it('should be an integer', async () => {
            rawUpdateConsensusParamsDocument.version = {
              appVersion: 'string',
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

          it('should be at least 1', async () => {
            rawUpdateConsensusParamsDocument.version = {
              appVersion: 0,
            };

            try {
              dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

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

        it('should be valid', async () => {
          const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

          const result = await dpp.document.validate(updateConsensusParams);

          expect(result.isValid()).to.be.true();
        });
      });

      it('should be valid', async () => {
        const updateConsensusParams = dpp.document.create(dataContract, identityId, 'updateConsensusParams', rawUpdateConsensusParamsDocument);

        const result = await dpp.document.validate(updateConsensusParams);

        expect(result.isValid()).to.be.true();
      });
    });

    describe('fixCumulativeFeesBug', () => {
      let rawFixCumulativeFeesBug;

      beforeEach(() => {
        rawFixCumulativeFeesBug = {
          enabled: true,
          enableAtHeight: 42,
        };
      });

      it('should not have additional properties', async () => {
        rawFixCumulativeFeesBug.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

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

      describe('enabled', () => {
        it('should be present', async () => {
          delete rawFixCumulativeFeesBug.enabled;

          try {
            dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('enabled');
          }
        });

        it('should be boolean', () => {
          rawFixCumulativeFeesBug.enabled = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('boolean');
          }
        });
      });

      describe('enabledAtHeight', () => {
        it('should be present', async () => {
          delete rawFixCumulativeFeesBug.enableAtHeight;

          try {
            dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('enableAtHeight');
          }
        });

        it('should be integer', () => {
          rawFixCumulativeFeesBug.enableAtHeight = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

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
          rawFixCumulativeFeesBug.enableAtHeight = 0;

          try {
            dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

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

      it('should be valid', async () => {
        const fixCumulativeFeesBug = dpp.document.create(dataContract, identityId, 'fixCumulativeFeesBug', rawFixCumulativeFeesBug);

        const result = await dpp.document.validate(fixCumulativeFeesBug);

        expect(result.isValid()).to.be.true();
      });
    });

    describe('verifyLLMQSignaturesWithCore', () => {
      let rawVerifyLLMQSignaturesWithCore;

      beforeEach(() => {
        rawVerifyLLMQSignaturesWithCore = {
          enabled: true,
          enableAtHeight: 42,
        };
      });

      it('should not have additional properties', async () => {
        rawVerifyLLMQSignaturesWithCore.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

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

      describe('enabled', () => {
        it('should be present', async () => {
          delete rawVerifyLLMQSignaturesWithCore.enabled;

          try {
            dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('enabled');
          }
        });

        it('should be boolean', () => {
          rawVerifyLLMQSignaturesWithCore.enabled = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('type');
            expect(error.params.type).to.equal('boolean');
          }
        });
      });

      describe('enabledAtHeight', () => {
        it('should be present', async () => {
          delete rawVerifyLLMQSignaturesWithCore.enableAtHeight;

          try {
            dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('enableAtHeight');
          }
        });

        it('should be integer', () => {
          rawVerifyLLMQSignaturesWithCore.enableAtHeight = 'string';

          try {
            dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

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
          rawVerifyLLMQSignaturesWithCore.enableAtHeight = 0;

          try {
            dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

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

      it('should be valid', async () => {
        const verifyLLMQSignaturesWithCore = dpp.document.create(dataContract, identityId, 'verifyLLMQSignaturesWithCore', rawVerifyLLMQSignaturesWithCore);

        const result = await dpp.document.validate(verifyLLMQSignaturesWithCore);

        expect(result.isValid()).to.be.true();
      });
    });
  });
});
