const crypto = require('crypto');

const generateRandomIdentifierAsync = require('../../../lib/test/utils/generateRandomIdentifierAsync');
const { default: loadWasmDpp, DocumentCreateTransition } = require('../../..');
const { getLatestProtocolVersion } = require('../../..');

let DataContractFactory;
let PlatformValueError;
let Identifier;
let ExtendedDocument;

// TODO: should be renamed to ExtendedDocument?
describe('Document', () => {
  let rawDocument;
  let document;
  let dataContract;
  let documentJs;
  let rawDocumentWithBuffers;

  // eslint-disable-next-line prefer-arrow-callback
  beforeEach(async function beforeEach() {
    ({
      Identifier, DataContractFactory, ExtendedDocument, PlatformValueError,
    } = await loadWasmDpp());

    const now = new Date().getTime();
    const id = await generateRandomIdentifierAsync();

    const ownerId = await generateRandomIdentifierAsync();
    const dataContractFactory = new DataContractFactory(
      1,
      { generate: () => crypto.randomBytes(32) },
    );

    const rawDataContract = {
      test: {
        type: 'object',
        properties: {
          name: {
            type: 'string',
            position: 0,
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
                    contentMediaType: Identifier.MEDIA_TYPE,
                    minItems: 32,
                    maxItems: 32,
                    position: 0,
                  },
                  binaryData: {
                    type: 'array',
                    byteArray: true,
                    minItems: 32,
                    maxItems: 32,
                    position: 1,
                  },
                },
                additionalProperties: false,
                position: 0,
              },
            },
            additionalProperties: false,
            position: 1,
          },
        },
        additionalProperties: false,
      },
    };

    // eslint-disable-next-line
    dataContract = dataContractFactory.create(ownerId, BigInt(1), rawDataContract);

    rawDocument = {
      $protocolVersion: getLatestProtocolVersion(),
      $id: id,
      $type: 'test',
      $dataContractId: dataContract.getId(),
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      $createdAt: now,
      $createdAtBlockHeight: 1,
      $createdAtCoreBlockHeight: 1,
      $updatedAt: now,
      $updatedAtBlockHeight: 1,
      $updatedAtCoreBlockHeight: 1,
      $transferredAt: null,
      $transferredAtBlockHeight: null,
      $transferredAtCoreBlockHeight: null,
    };

    rawDocumentWithBuffers = {
      $protocolVersion: getLatestProtocolVersion(),
      $id: id.toBuffer(),
      $type: 'test',
      $dataContractId: dataContract.getId().toBuffer(),
      $ownerId: ownerId.toBuffer(),
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      $createdAt: now,
      $createdAtBlockHeight: 1,
      $createdAtCoreBlockHeight: 1,
      $updatedAt: now,
      $updatedAtBlockHeight: 1,
      $updatedAtCoreBlockHeight: 1,
      $transferredAt: null,
      $transferredAtBlockHeight: null,
      $transferredAtCoreBlockHeight: null,
    };

    document = new ExtendedDocument(rawDocument, dataContract);
  });

  describe('constructor', () => {
    it('should create ExtendedDocument with $id and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $ownerId: await generateRandomIdentifierAsync(),
        $id: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      document = new ExtendedDocument(rawDocument, dataContract);
      expect(document.getId().toBuffer()).to.deep.equal(rawDocument.$id.toBuffer());
    });

    it.skip('should create DocumentCreateTransition with $type and data if present', () => {
      // const data = {
      //   test: 1,
      // };
      //
      // rawDocument = {
      //   $type: 'test',
      //   ...data,
      // };
      //
      // document = new DocumentCreateTransition(rawDocument, dataContract);
      //
      // expect(document.getType()).to.equal(rawDocument.$type);
    });

    it('should not create ExtendedDocument if $ownerId is missing', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      try {
        document = new ExtendedDocument(rawDocument, dataContract);
      } catch (e) {
        expect(e).to.be.instanceOf(PlatformValueError);
        expect(e.getMessage()).to.equal('serde deserialization error: missing field `$ownerId`');
      }
    });

    it('should not create ExtendedDocument if $id is missing', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $ownerId: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      try {
        document = new ExtendedDocument(rawDocument, dataContract);
      } catch (e) {
        expect(e).to.be.instanceOf(PlatformValueError);
        expect(e.getMessage()).to.equal('serde deserialization error: missing field `$id`');
      }
    });

    it('should not create ExtendedDocument if $type is missing', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        ...data,
      };

      try {
        document = new ExtendedDocument(rawDocument, dataContract);
      } catch (e) {
        expect(e).to.be.instanceOf(PlatformValueError);
        expect(e.getMessage()).to.equal('structure error: unable to remove string property $type');
      }
    });

    it('should not create ExtendedDocument if $dataContractId is missing', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      try {
        document = new ExtendedDocument(rawDocument, dataContract);
      } catch (e) {
        expect(e).to.be.instanceOf(PlatformValueError);
        expect(e.getMessage()).to.equal('structure error: unable to remove hash256 property $dataContractId');
      }
    });

    it('should create Document with undefined action and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        $type: 'test',
        ...data,
      };

      document = new ExtendedDocument(rawDocument, dataContract);
      expect(document.get('action')).to.equal(undefined);
    });

    it('should create Document with $revision and data if present', async () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        $revision: 123,
        $type: 'test',
        ...data,
      };

      document = new ExtendedDocument(rawDocument, dataContract);

      expect(document.getRevision()).to.equal(rawDocument.$revision);
    });

    it('should create Document with $createdAt and data if present', async () => {
      const data = {
        test: 1,
      };

      const createdAt = new Date().getTime();

      rawDocument = {
        $id: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $dataContractId: await generateRandomIdentifierAsync(),
        $createdAt: createdAt,
        $type: 'test',
        ...data,
      };

      document = new ExtendedDocument(rawDocument, dataContract);

      expect(document.getCreatedAt().getTime()).to.equal(rawDocument.$createdAt);
    });

    it('should create Document with $updatedAt and data if present', async () => {
      const data = {
        test: 1,
      };

      const updatedAt = new Date().getTime();

      rawDocument = {
        $dataContractId: await generateRandomIdentifierAsync(),
        $ownerId: await generateRandomIdentifierAsync(),
        $id: await generateRandomIdentifierAsync(),
        $updatedAt: updatedAt,
        $transferredAt: null,
        $transferredAtBlockHeight: null,
        $transferredAtCoreBlockHeight: null,
        $type: 'test',
        ...data,
      };

      document = new ExtendedDocument(rawDocument, dataContract);

      expect(document.getUpdatedAt().getTime()).to.equal(rawDocument.$updatedAt);
    });
  });

  describe('#getId', () => {
    it('should return ID', async () => {
      const id = await generateRandomIdentifierAsync();

      document.setId(id);

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

  describe('#setData/#getData', () => {
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

    // It should return Identifier instead of string
    it.skip('should set identifier as part of object', () => {
      const buffer = Buffer.alloc(32, 'a');
      const path = 'dataObject.binaryObject';
      const identifierPath = 'dataObject.binaryObject.identifier';
      const identifier = new Identifier(buffer);
      const value = { identifier };

      document.set(path, value);
      const returnedIdentifier = document.get(identifierPath);

      expect(returnedIdentifier.toBuffer()).to.deep.equal(buffer);
    });
  });

  describe('#toJSON', () => {
    it('should return Document as plain JS object', () => {
      const jsonDocument = {
        ...rawDocument,
        $dataContractId: document.getDataContractId().toString(),
        $dataContract: document.getDataContract().toJSON(),
        $id: document.getId().toString(),
        $ownerId: document.getOwnerId().toString(),
      };

      expect(document.toJSON()).to.deep.equal(jsonDocument);
    });
  });

  describe('#toObject', () => {
    it('should return Document as object', () => {
      const result = document.toObject();

      expect(rawDocumentWithBuffers).to.deep.equal(result);
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized Document', () => {
      const buffer = document.toBuffer();
      expect(buffer).to.be.instanceOf(Buffer);
      expect(buffer.length).to.equal(649);
    });

    // TODO: remove or replace?
    //  can not be compared to JS buffers anymore because uses bin code
    it.skip('should return the same bytes as JS version when dynamic identifier is in Document', () => {
      // const jsId = generateRandomIdentifier();
      // const id = new Identifier(jsId.toBuffer());
      // const path = 'dataObject.binaryObject.identifier';
      //
      // documentJs.set(path, jsId);
      // document.set(path, id);
      //
      // const documentJsIdBuffer = documentJs.get(path).toBuffer();
      // const documentIdBuffer = document.get(path).toBuffer();
      //
      // expect(documentJsIdBuffer).to.deep.equal(jsId);
      // expect(documentIdBuffer).to.deep.equal(jsId);
      //
      // const jsBuffer = documentJs.toBuffer();
      // const buffer = document.toBuffer();
      // expect(jsBuffer).to.deep.equal(buffer);
    });

    // TODO: remove or replace?
    //  can not be compared to JS buffers anymore because uses bin code
    it.skip('should return the same bytes as JS version when dynamic binaryData is in Document', () => {
      const data = Buffer.alloc(32);
      const path = 'dataObject.binaryObject.binaryData';

      documentJs.set(path, data);
      document.set(path, data);

      const jsBuffer = documentJs.toBuffer();
      const buffer = document.toBuffer();

      expect(jsBuffer.length).to.equal(buffer.length);
      expect(jsBuffer).to.deep.equal(buffer);
    });
  });

  describe.skip('#hash', () => {
    it('returned hash should be the same as JS version', () => {
      expect(documentJs.hash()).to.deep.equal(document.hash());
    });
  });

  describe('#setCreatedAt', () => {
    it('should set $createdAt', () => {
      const time = new Date();

      document.setCreatedAt(time);

      expect(document.getCreatedAt()).to.deep.equal(time);
    });
  });

  describe('#getCreatedAt', () => {
    it('should return $createdAt', () => {
      const time = new Date();

      document.setCreatedAt(time);

      expect(document.getCreatedAt()).to.deep.equal(time);
    });
  });

  describe('#setUpdatedAt', () => {
    it('should set $updatedAt', () => {
      const time = new Date();

      document.setUpdatedAt(time);

      expect(document.getUpdatedAt()).to.deep.equal(time);
    });
  });

  describe('#getUpdatedAt', () => {
    it('should return $updatedAt', () => {
      const time = new Date();

      document.setUpdatedAt(time);

      expect(document.getUpdatedAt()).to.deep.equal(time);
    });
  });
});
