const DataContractFactory = require('../../../lib/dataContract/DataContractFactory');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');

const DocumentCreateTransition = require(
  '../../../lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition',
);

const Identifier = require('../../../lib/identifier/Identifier');

const protocolVersion = require('../../../lib/version/protocolVersion');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');

const Document = require('../../../lib/document/Document');

const hash = require('../../../lib/util/hash');
const serializer = require('../../../lib/util/serializer');

describe('Document', () => {
  let hashMock;
  let encodeMock;
  let rawDocument;
  let document;
  let dataContract;

  beforeEach(function beforeEach() {
    const now = new Date().getTime();

    const ownerId = generateRandomIdentifier().toBuffer();

    const dataContractFactory = new DataContractFactory(createDPPMock(), () => {});

    dataContract = dataContractFactory.create(ownerId, {
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
                    contentMediaType: Identifier.MEDIA_TYPE,
                    minItems: 32,
                    maxItems: 32,
                  },
                },
              },
            },
          },
        },
      },
    });

    rawDocument = {
      $protocolVersion: protocolVersion.latestVersion,
      $id: generateRandomIdentifier(),
      $type: 'test',
      $dataContractId: dataContract.getId(),
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      $createdAt: now,
      $updatedAt: now,
    };

    document = new Document(rawDocument, dataContract);

    encodeMock = this.sinonSandbox.stub(serializer, 'encode');
    hashMock = this.sinonSandbox.stub(hash, 'hash');
  });

  afterEach(() => {
    encodeMock.restore();
    hashMock.restore();
  });

  describe('constructor', () => {
    it('should create Document with $id and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $id: Buffer.alloc(32),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.id).to.deep.equal(rawDocument.$id);
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

      expect(document.type).to.equal(rawDocument.$type);
    });

    it('should create Document with $dataContractId and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $dataContractId: generateRandomIdentifier().toBuffer(),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.dataContractId).to.deep.equal(rawDocument.$dataContractId);
    });

    it('should create Document with $ownerId and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $ownerId: generateRandomIdentifier().toBuffer(),
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.ownerId.toBuffer()).to.deep.equal(rawDocument.$ownerId);
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

      expect(document.action).to.equal(undefined);
    });

    it('should create Document with $revision and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $revision: 'test',
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument, dataContract);

      expect(document.revision).to.equal(rawDocument.$revision);
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

      expect(document.getCreatedAt().getTime()).to.equal(rawDocument.$createdAt);
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

      expect(document.getUpdatedAt().getTime()).to.equal(rawDocument.$updatedAt);
    });
  });

  describe('#getId', () => {
    it('should return ID', () => {
      const id = '123';

      document.id = id;

      const actualId = document.getId();

      expect(hashMock).to.have.not.been.called();

      expect(id).to.equal(actualId);
    });
  });

  describe('#getType', () => {
    it('should return $type', () => {
      expect(document.getType()).to.equal(rawDocument.$type);
    });
  });

  describe('#getOwnerId', () => {
    it('should return $ownerId', () => {
      expect(document.getOwnerId()).to.deep.equal(rawDocument.$ownerId);
    });
  });

  describe('#getDataContractId', () => {
    it('should return $dataContractId', () => {
      expect(document.getOwnerId()).to.deep.equal(rawDocument.$ownerId);
    });
  });

  describe('#setRevision', () => {
    it('should set $revision', () => {
      const revision = 5;

      const result = document.setRevision(revision);

      expect(result).to.equal(document);

      expect(document.revision).to.equal(revision);
    });
  });

  describe('#getRevision', () => {
    it('should return $revision', () => {
      const revision = 5;

      document.revision = revision;

      expect(document.getRevision()).to.equal(revision);
    });
  });

  describe('#setData', () => {
    it('should call set for each document property', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      const result = document.setData(data);

      expect(result).to.equal(document);
    });
  });

  describe('#getData', () => {
    it('should return all data', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      document.data = data;

      expect(document.getData()).to.equal(data);
    });
  });

  describe('#set', () => {
    it('should set value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      const result = document.set(path, value);

      expect(result).to.equal(document);
    });

    it('should set identifier', () => {
      const path = 'dataObject.binaryObject.identifier';
      const buffer = Buffer.alloc(32);

      const result = document.set(path, buffer);

      expect(result).to.equal(document);
    });

    it('should set identifier as part of object', () => {
      const buffer = Buffer.alloc(32, 'a');
      const path = 'dataObject.binaryObject';
      const value = { identifier: buffer };

      const result = document.set(path, value);

      expect(result).to.equal(document);
    });
  });

  describe('#get', () => {
    it('should return value for specified property name', () => {
      const path = 'dataObject.binaryObject.identifier';
      const buffer = Buffer.alloc(32);

      document.set(path, buffer);

      const result = document.get(path);

      expect(result).to.deep.equal(buffer);
    });
  });

  describe('#toJSON', () => {
    it('should return Document as plain JS object', () => {
      const jsonDocument = {
        ...rawDocument,
        $dataContractId: document.dataContractId.toString(),
        $id: document.id.toString(),
        $ownerId: document.ownerId.toString(),
      };

      expect(document.toJSON()).to.deep.equal(jsonDocument);
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized Document', () => {
      const serializedDocument = Buffer.from('123');

      encodeMock.returns(serializedDocument);

      const result = document.toBuffer();

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(rawDocument.$protocolVersion, 0);

      expect(result).to.deep.equal(Buffer.concat([protocolVersionUInt32, serializedDocument]));

      const documentToEncode = { ...rawDocument };
      delete documentToEncode.$protocolVersion;

      expect(encodeMock.getCall(0).args).to.have.deep.members([
        documentToEncode,
      ]);
    });
  });

  describe('#hash', () => {
    let toBufferMock;

    beforeEach(function beforeEach() {
      toBufferMock = this.sinonSandbox.stub(Document.prototype, 'toBuffer');
    });

    afterEach(() => {
      toBufferMock.restore();
    });

    it('should return Document hash', () => {
      const serializedDocument = '123';
      const hashedDocument = '456';

      toBufferMock.returns(serializedDocument);

      hashMock.returns(hashedDocument);

      const result = document.hash();

      expect(result).to.equal(hashedDocument);

      expect(toBufferMock).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });

  describe('#setCreatedAt', () => {
    it('should set $createdAt', () => {
      const time = new Date();

      const result = document.setCreatedAt(time);

      expect(result).to.equal(document);

      expect(document.createdAt).to.equal(time);
    });
  });

  describe('#getCreatedAt', () => {
    it('should return $createdAt', () => {
      const time = new Date();

      document.createdAt = time;

      expect(document.getCreatedAt()).to.equal(time);
    });
  });

  describe('#setUpdatedAt', () => {
    it('should set $updatedAt', () => {
      const time = new Date();

      const result = document.setUpdatedAt(time);

      expect(result).to.equal(document);

      expect(document.updatedAt).to.equal(time);
    });
  });

  describe('#getUpdatedAt', () => {
    it('should return $updatedAt', () => {
      const time = new Date();

      document.updatedAt = time;

      expect(document.getUpdatedAt()).to.equal(time);
    });
  });
});
