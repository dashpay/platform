const bs58 = require('bs58');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifierAsync');

const { default: loadWasmDpp } = require('../../../dist');

describe('DataContract', () => {
  let documentType;
  let documentSchema;
  let documentSchemas;
  let ownerId;
  let identityNonce;
  let contractId;
  let dataContract;
  let schemaDefs;

  let DataContract;
  let Identifier;
  let Metadata;

  before(async () => {
    ({
      DataContract, Identifier, Metadata,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    documentType = 'niceDocument';

    documentSchema = {
      type: 'object',
      properties: {
        nice: {
          type: 'boolean',
          position: 0,
        },
        aBinaryProperty: {
          type: 'array',
          byteArray: true,
          position: 1,
        },
      },
      additionalProperties: false,
    };

    documentSchemas = {
      [documentType]: documentSchema,
    };

    ownerId = (await generateRandomIdentifier()).toBuffer();
    // eslint-disable-next-line
    identityNonce = BigInt(1);
    contractId = (await generateRandomIdentifier()).toBuffer();

    schemaDefs = { something: { type: 'string' } };

    dataContract = new DataContract({
      $format_version: '0',
      id: contractId,
      version: 1,
      ownerId,
      documentSchemas,
    });
  });

  describe('constructor', () => {
    it('should create new DataContract', async () => {
      dataContract = new DataContract({
        $format_version: '0',
        id: contractId,
        version: 1,
        ownerId,
        documentSchemas,
        schemaDefs,
      });

      expect(dataContract.getId().toBuffer()).to.deep.equal(contractId);
      expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(ownerId);
      expect(dataContract.getDocumentSchemas()).to.deep.equal(documentSchemas);
      expect(dataContract.getSchemaDefs()).to.deep.equal(schemaDefs);
    });
  });

  describe('#getId', () => {
    it('should return DataContract Identifier', () => {
      const result = dataContract.getId();

      expect(result.toBuffer()).to.deep.equal(contractId);
      expect(result).to.be.instanceof(Identifier);
    });
  });

  describe('#getDocumentSchemas', () => {
    it('should get Documents definition', () => {
      const anotherDocuments = {
        anotherDocument: {
          type: 'object',
          properties: {
            name: {
              type: 'string',
              position: 0,
            },
          },
          additionalProperties: false,
        },
      };

      dataContract.setDocumentSchemas(anotherDocuments);
      expect(dataContract.getDocumentSchemas()).to.deep.equal(anotherDocuments);
    });
  });

  describe('#setDocumentSchemas', () => {
    it('should set Documents definition', () => {
      const anotherDocuments = {
        anotherDocument: {
          type: 'object',
          properties: {
            name: {
              type: 'string',
              position: 0,
            },
          },
          additionalProperties: false,
        },
      };

      dataContract.setDocumentSchemas(anotherDocuments);
      expect(dataContract.getDocumentSchemas()).to.deep.equal(anotherDocuments);
    });
  });

  describe('#hasDocumentType', () => {
    it('should return true if Document schema is defined', () => {
      const result = dataContract.hasDocumentType('niceDocument');

      expect(result).to.equal(true);
    });

    it('should return false if Document schema is not defined', () => {
      const result = dataContract.hasDocumentType('undefinedDocument');

      expect(result).to.equal(false);
    });
  });

  describe('#setDocumentSchema', () => {
    it('should set Document schema', () => {
      const anotherType = 'prettyDocument';
      const anotherDefinition = {
        type: 'object',
        properties: {
          test: {
            type: 'string',
            position: 0,
          },
        },
        additionalProperties: false,
      };

      dataContract.setDocumentSchema(anotherType, anotherDefinition);

      const anotherDocuments = dataContract.getDocumentSchemas();

      expect(anotherDocuments).to.have.property(anotherType);
      expect(anotherDocuments[anotherType]).to.deep.equal(anotherDefinition);
      expect(dataContract.hasDocumentType(anotherType)).to.be.true();
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      try {
        dataContract.getDocumentSchema('undefinedObject');
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.getMessage()).to.equal('data contract error: document type not found: can not get document type from contract');
      }
    });

    it('should return Document Schema', () => {
      const result = dataContract.getDocumentSchema(documentType);

      expect(result).to.deep.equal(documentSchema);
    });
  });

  describe('#setSchemaDefs', () => {
    it('should set $defs', () => {
      const $defs = {
        subSchema: {
          type: 'object',
          properties: {
            test: {
              type: 'string',
              position: 0,
            },
          },
          additionalProperties: false,
        },
      };

      dataContract.setSchemaDefs($defs);

      expect(dataContract.getSchemaDefs()).to.deep.equal($defs);
    });
  });

  describe('#getSchemaDefs', () => {
    it('should return $defs', () => {
      const result = dataContract.getSchemaDefs();

      expect(result).to.be.null();
    });
  });

  describe('#toJSON', () => {
    it('should return JataContract as plain object', () => {
      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        $format_version: '0',
        config: {
          $format_version: '0',
          canBeDeleted: false,
          documentsCanBeDeletedContractDefault: true,
          documentsKeepHistoryContractDefault: false,
          documentsMutableContractDefault: true,
          keepsHistory: false,
          readonly: false,
          requiresIdentityDecryptionBoundedKey: null,
          requiresIdentityEncryptionBoundedKey: null,
        },
        id: bs58.encode(contractId),
        version: 1,
        ownerId: bs58.encode(ownerId),
        schemaDefs: null,
        documentSchemas,
      });
    });

    it('should return plain object with "$defs" if present', () => {
      const $defs = {
        subSchema: { type: 'string' },
      };

      dataContract.setSchemaDefs($defs);

      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        $format_version: '0',
        config: {
          $format_version: '0',
          canBeDeleted: false,
          documentsCanBeDeletedContractDefault: true,
          documentsKeepHistoryContractDefault: false,
          documentsMutableContractDefault: true,
          keepsHistory: false,
          readonly: false,
          requiresIdentityDecryptionBoundedKey: null,
          requiresIdentityEncryptionBoundedKey: null,
        },
        id: bs58.encode(contractId),
        version: 1,
        ownerId: bs58.encode(ownerId),
        documentSchemas,
        schemaDefs: $defs,
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return DataContract as a Buffer', () => {
      const result = dataContract.toBuffer();
      expect(result).to.be.instanceOf(Buffer);
      expect(result).to.have.lengthOf(236);
    });
  });

  // TODO: can not compare to JS because rust
  //  DataContract does not match JS anymore
  describe('#hash', () => {
    it('should return DataContract hash', () => {
      // const jsResult = jsDataContract.hash();
      const wasmResult = dataContract.hash();
      //
      // expect(wasmResult).to.deep.equal(jsResult);
      expect(wasmResult).to.be.instanceOf(Uint8Array);
    });
  });

  describe('#setIdentityNonce', () => {
    it('should set entropy', () => {
      dataContract.setIdentityNonce(identityNonce);

      expect(dataContract.getIdentityNonce()).to.deep.equal(identityNonce);
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata({
        blockHeight: 43,
        coreChainLockedHeight: 1,
        timeMs: 100,
        protocolVersion: 2,
      });
      const otherMetadataToObject = otherMetadata.toObject();

      dataContract.setMetadata(otherMetadata);

      expect(dataContract.getMetadata().toObject()).to.deep.equal(otherMetadataToObject);
    });
  });

  describe.skip('#setConfig', () => {
    it('should set config', () => {
      const config = {
        allow: true,
        readonly: true,
        keepsHistory: true,
        documentsKeepHistoryContractDefault: true,
        documentsMutableContractDefault: true,
      };

      dataContract.setConfig(config);

      const restoredConfig = dataContract.getConfig();

      expect(config).to.deep.equal(restoredConfig);
    });
  });
});
