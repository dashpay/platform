const rewiremock = require('rewiremock/node');

const DataContractFactory = require('../../../lib/dataContract/DataContractFactory');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');

const DocumentCreateTransition = require(
  '../../../lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition',
);

const Identifier = require('../../../lib/identifier/Identifier');

const { protocolVersion } = require('../../../lib/protocolVersion');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');

describe('Document', () => {
  let lodashGetMock;
  let lodashSetMock;
  let lodashCloneDeepMock;
  let hashMock;
  let encodeMock;
  let Document;
  let rawDocument;
  let document;
  let dataContract;

  beforeEach(function beforeEach() {
    lodashGetMock = this.sinonSandbox.stub();
    lodashSetMock = this.sinonSandbox.stub();
    lodashCloneDeepMock = this.sinonSandbox.stub().returnsArg(0);
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    const now = new Date().getTime();

    Document = rewiremock.proxy('../../../lib/document/Document', {
      '../../../node_modules/lodash.get': lodashGetMock,
      '../../../node_modules/lodash.set': lodashSetMock,
      '../../../node_modules/lodash.clonedeep': lodashCloneDeepMock,
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

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
      $protocolVersion: protocolVersion,
      $id: generateRandomIdentifier(),
      $type: 'test',
      $dataContractId: dataContract.getId(),
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      $createdAt: now,
      $updatedAt: now,
    };

    document = new Document(rawDocument, dataContract);
  });

  describe('constructor', () => {
    beforeEach(function beforeEach() {
      Document.prototype.setData = this.sinonSandbox.stub();
    });

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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
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
    beforeEach(function beforeEach() {
      Document.prototype.set = this.sinonSandbox.stub();
    });

    it('should call set for each document property', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      const result = document.setData(data);

      expect(result).to.equal(document);

      expect(Document.prototype.set).to.have.been.calledTwice();

      expect(Document.prototype.set).to.have.been.calledWith('test1', 1);
      expect(Document.prototype.set).to.have.been.calledWith('test2', 2);
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

      expect(lodashSetMock).to.have.been.calledOnceWith(document.data, path, value);
    });

    it('should set identifier', () => {
      const path = 'dataObject.binaryObject.identifier';
      const buffer = Buffer.alloc(32);

      const result = document.set(path, buffer);

      expect(result).to.equal(document);

      expect(lodashSetMock).to.have.been.calledOnce();

      expect(lodashSetMock.getCall(0).args).to.have.deep.members(
        [document.data, path, buffer],
      );

      expect(lodashSetMock.getCall(0).args[2]).to.be.instanceof(Identifier);
    });

    it('should set identifier as part of object', () => {
      const buffer = Buffer.alloc(32, 'a');
      const path = 'dataObject.binaryObject';
      const value = { identifier: buffer };

      lodashGetMock.returns(value.identifier);

      const result = document.set(path, value);

      expect(result).to.equal(document);

      expect(lodashSetMock).to.have.been.calledTwice();

      expect(lodashSetMock.getCall(0).args).to.have.deep.members(
        [value, 'identifier', buffer],
      );

      expect(lodashSetMock.getCall(0).args[2]).to.be.instanceof(Identifier);

      expect(lodashSetMock.getCall(1).args).to.have.deep.members(
        [document.data, path, value],
      );
    });
  });

  describe('#get', () => {
    it('should return value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      lodashGetMock.returns(value);

      const result = document.get(path);

      expect(result).to.equal(value);

      expect(lodashGetMock).to.have.been.calledOnceWith(document.data, path);
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
      protocolVersionUInt32.writeUInt32BE(rawDocument.$protocolVersion, 0);

      expect(result).to.deep.equal(Buffer.concat([protocolVersionUInt32, serializedDocument]));

      const documentToEncode = { ...rawDocument };
      delete documentToEncode.$protocolVersion;

      expect(encodeMock.getCall(0).args).to.have.deep.members([
        documentToEncode,
      ]);
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      Document.prototype.toBuffer = this.sinonSandbox.stub();
    });

    it('should return Document hash', () => {
      const serializedDocument = '123';
      const hashedDocument = '456';

      Document.prototype.toBuffer.returns(serializedDocument);

      hashMock.returns(hashedDocument);

      const result = document.hash();

      expect(result).to.equal(hashedDocument);

      expect(Document.prototype.toBuffer).to.have.been.calledOnce();

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
