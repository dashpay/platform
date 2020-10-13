const crypto = require('crypto');

const DashPlatformProtocol = require('@dashevo/dpp');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const dpnsContractDocumentsSchema = require('../../schema/dpns-contract-documents.json');

describe('DPNS Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(function beforeEach() {
    const fetchContractStub = this.sinon.stub();

    dpp = new DashPlatformProtocol({
      stateRepository: {
        fetchDataContract: fetchContractStub,
      },
    });

    identityId = generateRandomIdentifier();

    dataContract = dpp.dataContract.create(identityId, dpnsContractDocumentsSchema);

    fetchContractStub.resolves(dataContract);
  });

  it('should have a valid contract definition', async () => {
    const validationResult = await dpp.dataContract.validate(dataContract);

    expect(validationResult.isValid()).to.be.true();
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

          try {
            dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('saltedDomainHash');
          }
        });

        it('should not be empty', async () => {
          rawPreorderDocument.saltedDomainHash = Buffer.alloc(0);

          try {
            dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minBytesLength');
            expect(error.dataPath).to.equal('.saltedDomainHash');
          }
        });

        it('should be not less than 32 bytes', async () => {
          rawPreorderDocument.saltedDomainHash = crypto.randomBytes(10);

          try {
            dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minBytesLength');
            expect(error.dataPath).to.equal('.saltedDomainHash');
          }
        });

        it('should be not longer than 32 bytes', async () => {
          rawPreorderDocument.saltedDomainHash = crypto.randomBytes(40);

          try {
            dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxBytesLength');
            expect(error.dataPath).to.equal('.saltedDomainHash');
          }
        });
      });

      it('should not have additional properties', async () => {
        rawPreorderDocument.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

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

      it('should be valid', async () => {
        const preorder = dpp.document.create(dataContract, identityId, 'preorder', rawPreorderDocument);

        const result = await dpp.document.validate(preorder);

        expect(result.isValid()).to.be.true();
      });
    });

    describe('domain', () => {
      let rawDomainDocument;

      beforeEach(() => {
        rawDomainDocument = {
          label: 'Wallet',
          normalizedLabel: 'wallet',
          normalizedParentDomainName: 'dash',
          preorderSalt: crypto.randomBytes(32),
          records: {
            dashUniqueIdentityId: generateRandomIdentifier(),
          },
          subdomainRules: {
            allowSubdomains: false,
          },
        };
      });

      describe('label', () => {
        it('should be present', async () => {
          delete rawDomainDocument.label;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('label');
          }
        });

        it('should follow pattern', async () => {
          rawDomainDocument.label = 'invalid label';

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('pattern');
            expect(error.dataPath).to.equal('.label');
          }
        });

        it('should be longer than 3 chars', async () => {
          rawDomainDocument.label = 'ab';

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minLength');
            expect(error.dataPath).to.equal('.label');
          }
        });

        it('should be less than 63 chars', async () => {
          rawDomainDocument.label = 'a'.repeat(64);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxLength');
            expect(error.dataPath).to.equal('.label');
          }
        });
      });

      describe('normalizedLabel', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.normalizedLabel;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('normalizedLabel');
          }
        });

        it('should follow pattern', async () => {
          rawDomainDocument.normalizedLabel = 'InValiD label';

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('pattern');
            expect(error.dataPath).to.equal('.normalizedLabel');
          }
        });

        it('should be less than 63 chars', async () => {
          rawDomainDocument.normalizedLabel = 'a'.repeat(64);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxLength');
            expect(error.dataPath).to.equal('.normalizedLabel');
          }
        });
      });

      describe('normalizedParentDomainName', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.normalizedParentDomainName;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('normalizedParentDomainName');
          }
        });

        it('should be less than 190 chars', async () => {
          rawDomainDocument.normalizedParentDomainName = 'a'.repeat(191);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxLength');
            expect(error.dataPath).to.equal('.normalizedParentDomainName');
          }
        });

        it('should follow pattern', async () => {
          rawDomainDocument.normalizedParentDomainName = '&'.repeat(100);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('pattern');
            expect(error.dataPath).to.equal('.normalizedParentDomainName');
          }
        });
      });

      describe('preorderSalt', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.preorderSalt;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('preorderSalt');
          }
        });

        it('should not be empty', async () => {
          rawDomainDocument.preorderSalt = Buffer.alloc(0);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minBytesLength');
            expect(error.dataPath).to.equal('.preorderSalt');
          }
        });

        it('should be not less than 32 bytes', async () => {
          rawDomainDocument.preorderSalt = crypto.randomBytes(10);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minBytesLength');
            expect(error.dataPath).to.equal('.preorderSalt');
          }
        });

        it('should be not longer than 32 bytes', async () => {
          rawDomainDocument.preorderSalt = crypto.randomBytes(40);

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('maxBytesLength');
            expect(error.dataPath).to.equal('.preorderSalt');
          }
        });
      });

      it('should not have additional properties', async () => {
        rawDomainDocument.someOtherProperty = 42;

        try {
          dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

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

      it('should be valid', async () => {
        const domain = dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.true();
      });

      describe('Records', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.records;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('records');
          }
        });

        it('should not be empty', async () => {
          rawDomainDocument.records = {};

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('minProperties');
            expect(error.dataPath).to.equal('.records');
          }
        });

        it('should not have additional properties', async () => {
          rawDomainDocument.records = {
            someOtherProperty: 42,
          };

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('additionalProperties');
            expect(error.dataPath).to.equal('.records');
            expect(error.params.additionalProperty).to.equal('someOtherProperty');
          }
        });

        describe('Dash Identity', () => {
          it('should have either `dashUniqueIdentityId` or `dashAliasIdentityId`', async () => {
            rawDomainDocument.records = {
              dashUniqueIdentityId: identityId,
              dashAliasIdentityId: identityId,
            };

            try {
              dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

              expect.fail('should throw error');
            } catch (e) {
              expect(e.name).to.equal('InvalidDocumentError');
              expect(e.getErrors()).to.have.a.lengthOf(1);

              const [error] = e.getErrors();

              expect(error.name).to.equal('JsonSchemaError');
              expect(error.keyword).to.equal('maxProperties');
              expect(error.dataPath).to.equal('.records');
            }
          });

          describe('dashUniqueIdentityId', () => {
            it('should no less than 32 bytes', async () => {
              rawDomainDocument.records = {
                dashUniqueIdentityId: crypto.randomBytes(30),
              };

              try {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('minBytesLength');
                expect(error.dataPath).to.equal('.records.dashUniqueIdentityId');
              }
            });

            it('should no more than 32 bytes', async () => {
              rawDomainDocument.records = {
                dashUniqueIdentityId: crypto.randomBytes(64),
              };

              try {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('maxBytesLength');
                expect(error.dataPath).to.equal('.records.dashUniqueIdentityId');
              }
            });
          });

          describe('dashAliasIdentityId', () => {
            it('should no less than 32 bytes', async () => {
              rawDomainDocument.records = {
                dashAliasIdentityId: crypto.randomBytes(30),
              };

              try {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('minBytesLength');
                expect(error.dataPath).to.equal('.records.dashAliasIdentityId');
              }
            });

            it('should no more than 32 bytes', async () => {
              rawDomainDocument.records = {
                dashAliasIdentityId: crypto.randomBytes(64),
              };

              try {
                dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

                expect.fail('should throw error');
              } catch (e) {
                expect(e.name).to.equal('InvalidDocumentError');
                expect(e.getErrors()).to.have.a.lengthOf(1);

                const [error] = e.getErrors();

                expect(error.name).to.equal('JsonSchemaError');
                expect(error.keyword).to.equal('maxBytesLength');
                expect(error.dataPath).to.equal('.records.dashAliasIdentityId');
              }
            });
          });
        });
      });

      describe('subdomainRules', () => {
        it('should be defined', async () => {
          delete rawDomainDocument.subdomainRules;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('required');
            expect(error.params.missingProperty).to.equal('subdomainRules');
          }
        });

        it('should not have additional properties', async () => {
          rawDomainDocument.subdomainRules.someOtherProperty = 42;

          try {
            dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

            expect.fail('should throw error');
          } catch (e) {
            expect(e.name).to.equal('InvalidDocumentError');
            expect(e.getErrors()).to.have.a.lengthOf(1);

            const [error] = e.getErrors();

            expect(error.name).to.equal('JsonSchemaError');
            expect(error.keyword).to.equal('additionalProperties');
            expect(error.dataPath).to.equal('.subdomainRules');
          }
        });

        describe('allowSubdomains', () => {
          it('should be boolean', async () => {
            rawDomainDocument.subdomainRules.allowSubdomains = 'data';

            try {
              dpp.document.create(dataContract, identityId, 'domain', rawDomainDocument);

              expect.fail('should throw error');
            } catch (e) {
              expect(e.name).to.equal('InvalidDocumentError');
              expect(e.getErrors()).to.have.a.lengthOf(1);

              const [error] = e.getErrors();

              expect(error.name).to.equal('JsonSchemaError');
              expect(error.keyword).to.equal('type');
              expect(error.dataPath).to.equal('.subdomainRules.allowSubdomains');
            }
          });
        });
      });
    });
  });
});
