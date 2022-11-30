const JsDataContractFactory = require('@dashevo/dpp/lib/dataContract/DataContractFactory');
const JsIdentifier = require('@dashevo/dpp/lib/identifier/Identifier');
const JsDocument = require('@dashevo/dpp/lib/document/Document');
const DocumentCreateTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition',
);
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const cloneDeepWith = require('lodash.clonedeep');

const generateRandomIdentifierAsync = require('../../../lib/test/utils/generateRandomIdentifierAsync');
const { default: loadWasmDpp } = require('../../../dist');

let DataContractFactory;
let Identifier;
let Document;

describe('Document', () => {
  let rawDocument;
  let document;
  let dataContract;
  let jsDocument;
  let jsDataContract;

  beforeEach(async () => {
    ({
      Identifier, Document, DataContractFactory,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    const now = new Date().getTime();
    const id = await generateRandomIdentifierAsync();
    const jsId = new JsIdentifier(Buffer.from(id.toBuffer()));

    const ownerId = await generateRandomIdentifierAsync();
    const jsOwnerId = new JsIdentifier(Buffer.from(ownerId.toBuffer()));

    const jsDataContractFactory = new JsDataContractFactory(createDPPMock(), () => { });
    const dataContractFactory = new DataContractFactory(1);
    const rawDataContract = {
      test: {
        properties: {
          name: {
            type: 'string',
          },
          dataObject: {
            type: 'object',
            properties: {
              binaryObject: {
                type: 'object',
                properties: {
                  identifier: {
                    type: 'array',
                    byteArray: true,
                    contentMediaType: JsIdentifier.MEDIA_TYPE,
                    minItems: 32,
                    maxItems: 32,
                  },
                },
              },
            },
          },
        },
      },
    };

    dataContract = dataContractFactory.create(ownerId.clone(), rawDataContract);
    jsDataContract = jsDataContractFactory.create(jsOwnerId, rawDataContract);

    rawDocument = {
      $protocolVersion: protocolVersion.latestVersion,
      $id: id,
      $type: 'test',
      $dataContractId: dataContract.getId(),
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      $createdAt: now,
      $updatedAt: now,
    };

    document = new Document(rawDocument, dataContract);
    const jsRawDocument = cloneDeepWith(rawDocument);
    jsRawDocument.$id = jsId;
    jsRawDocument.$ownerId = jsOwnerId;

    jsRawDocument.$dataContractId = jsDataContract.id;
    jsDocument = new JsDocument(jsRawDocument, jsDataContract);
    jsDocument.dataContractId = JsIdentifier.from(Buffer.from(dataContract.getId().toBuffer()));
  });

  describe('constructor', () => {
    it('should create Document with $id and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);
      expect(document.getId().toBuffer()).to.deep.equal(rawDocument.$id.toBuffer());
    });

    it('should create Document with $type and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getType()).to.equal(rawDocument.$type);
    });

    it('should create Document with $dataContractId and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $dataContractId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getDataContractId().toBuffer())
        .to.deep.equal(rawDocument.$dataContractId.toBuffer());
    });

    it('should create Document with $ownerId and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $ownerId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getOwnerId().toBuffer()).to.deep.equal(rawDocument.$ownerId.toBuffer());
    });

    it('should create Document with undefined action and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);
      expect(document.get('action')).to.equal(undefined);
    });

    it('should create Document with $revision and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $revision: 123,
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getRevision()).to.equal(rawDocument.$revision);
    });

    it('should create Document with $createdAt and data if present', async () => {
      const data = {
        test: 1,
      };

      const createdAt = new Date().getTime();

      rawDocument = {
        $createdAt: createdAt,
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getCreatedAt()).to.equal(rawDocument.$createdAt);
    });

    it('should create Document with $updatedAt and data if present', async () => {
      const data = {
        test: 1,
      };

      const updatedAt = new Date().getTime();

      rawDocument = {
        $updatedAt: updatedAt,
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.getUpdatedAt()).to.equal(rawDocument.$updatedAt);
    });

    describe('#getId', () => {
      it('should return ID', async () => {
        const id = await generateRandomIdentifierAsync();

        document.setId(id.clone());

        const actualId = document.getId();

        expect(id.toBuffer()).to.deep.equal(actualId.toBuffer());
      });
    });

    describe('#getType', () => {
      it('should return $type', () => {
        expect(document.getType()).to.equal(rawDocument.$type);
      });
    });

    describe('#getOwnerId', () => {
      it('should return $ownerId', () => {
        expect(document.getOwnerId().toBuffer()).to.deep.equal(rawDocument.$ownerId.toBuffer());
      });
    });

    describe('#getDataContractId', () => {
      it('should return $dataContractId', () => {
        expect(document.getOwnerId().toBuffer()).to.deep.equal(rawDocument.$ownerId.toBuffer());
      });
    });

    describe('#setRevision/#getRevision', () => {
      it('should set $revision and get $revision', () => {
        const revision = 5;

        document.setRevision(revision);

        expect(document.getRevision()).to.equal(revision);
      });
    });

    describe('#setData/#getDAta', () => {
      it('should call set and get for each document property', () => {
        const data = {
          test1: 1,
          test2: 2,
        };

        document.setData(data);

        expect(document.getData()).to.deep.equal(data);
      });
    });

    describe('#set', () => {
      it('should set value for specified property name', () => {
        const path = 'test[0].$my';
        const value = 2;

        document.set(path, value);

        expect(document.get(path)).to.deep.equal(2);
      });

      it('should set identifier', () => {
        const path = 'dataObject.binaryObject.identifier';
        const buffer = Buffer.alloc(32);
        const identifier = new Identifier(buffer);

        document.set(path, identifier);

        expect(document.get(path).toBuffer()).to.deep.equal(buffer);
      });

      it('should set identifier as part of object', () => {
        const buffer = Buffer.alloc(32, 'a');
        const path = 'dataObject.binaryObject';
        const identifierPath = 'dataObject.binaryObject.identifier';
        const identifier = new Identifier(buffer);
        const value = { identifier };

        document.set(path, value);

        expect(document.get(identifierPath).toBuffer()).to.deep.equal(buffer);
      });
    });

    describe('#toJSON', () => {
      it('should return Document as plain JS object', () => {
        const jsonDocument = {
          ...rawDocument,
          $dataContractId: document.getDataContractId().toString(),
          $id: document.getId().toString(),
          $ownerId: document.getOwnerId().toString(),
        };

        expect(document.toJSON()).to.deep.equal(jsonDocument);
      });
    });

    describe('#toObject', () => {
      it('should return Document as object', () => {
        const result = document.toObject();

        // as we can't compare Identifiers from wasm, therefore we verify every field explicitly
        expect(result.$protocolVersion).to.deep.equal(rawDocument.$protocolVersion);
        expect(result.$type).to.deep.equal(rawDocument.$type);
        expect(result.$revision).to.deep.equal(rawDocument.$revision);
        expect(result.$createdAt).to.deep.equal(rawDocument.$createdAt);
        expect(result.$updatedAt).to.deep.equal(rawDocument.$updatedAt);
        expect(result.$id.toBuffer()).to.deep.equal(rawDocument.$id.toBuffer());
        expect(result.$dataContractId.toBuffer())
          .to.deep.equal(rawDocument.$dataContractId.toBuffer());
        expect(result.$ownerId.toBuffer())
          .to.deep.equal(rawDocument.$ownerId.toBuffer());
      });
    });

    describe('#toBuffer', () => {
      it('returned bytes should be the same as JS version', () => {
        const jsBuffer = jsDocument.toBuffer();
        const buffer = document.toBuffer();

        expect(jsBuffer.length).to.equal(buffer.length);
        expect(jsBuffer).to.deep.equal(buffer);
      });

      // TODO fixme
      // it('should return the same bytes as JS version when dynamic identifier is in Document', () => {

      //   // we should set the identifier
      //   const jsId = new JsIdentifier.from(Buffer.alloc(32));
      //   const id = new Identifier(Buffer.alloc(32));
      //   const path = "dataObject.binaryObject.identifier";


      //   jsDocument.set(path, jsId);
      //   document.set(path, id);

      //   console.log(jsDocument);
      //   console.log("-----------------------")
      //   console.log(document.toObject());

      //   const jsBuffer = jsDocument.toBuffer();
      //   const buffer = document.toBuffer();

      //   expect(jsBuffer.length).to.equal(buffer.length);
      //   expect(jsBuffer).to.deep.equal(buffer);
      // });
    });

    describe('#hash', () => {
      it('returned hash should be the same as JS version', () => {
        expect(jsDocument.hash()).to.deep.equal(document.hash());
      });
    });

    describe('#setCreatedAt', () => {
      it('should set $createdAt', () => {
        const time = new Date().getTime();

        document.setCreatedAt(time);

        expect(document.getCreatedAt()).to.equal(time);
      });
    });

    describe('#getCreatedAt', () => {
      it('should return $createdAt', () => {
        const time = new Date().getTime();

        document.setCreatedAt(time);

        expect(document.getCreatedAt()).to.equal(time);
      });
    });

    describe('#setUpdatedAt', () => {
      it('should set $updatedAt', () => {
        const time = new Date().getTime();

        document.setUpdatedAt(time);

        expect(document.getUpdatedAt()).to.equal(time);
      });
    });

    describe('#getUpdatedAt', () => {
      it('should return $updatedAt', () => {
        const time = new Date().getTime();

        document.setUpdatedAt(time);

        expect(document.getUpdatedAt()).to.equal(time);
      });
    });
  });
});
