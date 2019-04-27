const bs58 = require('bs58');
const rewiremock = require('rewiremock/node');

const DataIsNotAllowedWithActionDeleteError = require('../../../lib/document/errors/DataIsNotAllowedWithActionDeleteError');

const DocumentMetadata = require('../../../lib/document/DocumentMetadata');

describe('Document', () => {
  let lodashGetMock;
  let lodashSetMock;
  let hashMock;
  let encodeMock;
  let Document;
  let rawDocument;
  let document;

  beforeEach(function beforeEach() {
    lodashGetMock = this.sinonSandbox.stub();
    lodashSetMock = this.sinonSandbox.stub();
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    Document = rewiremock.proxy('../../../lib/document/Document', {
      '../../../node_modules/lodash.get': lodashGetMock,
      '../../../node_modules/lodash.set': lodashSetMock,
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    rawDocument = {
      $type: 'test',
      $scope: 'a832e4145650bfe8462e768e9c4a9a0d3a0bb7dcd9b3e50c61c73ac9d2e14068',
      $scopeId: 'ydhM7GjG4QUbcuXpZDVoi7TTn7LL8Rhgzh',
      $action: Document.DEFAULTS.ACTION,
      $rev: Document.DEFAULTS.REVISION,
    };

    document = new Document(rawDocument);
  });

  describe('constructor', () => {
    beforeEach(function beforeEach() {
      Document.prototype.setData = this.sinonSandbox.stub();
    });

    it('should create Document with $type and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $type: 'test',
        ...data,
      };

      document = new Document(rawDocument);

      expect(document.type).to.equal(rawDocument.$type);
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
    });

    it('should create Document with $scopeId and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $scopeId: 'test',
        ...data,
      };

      document = new Document(rawDocument);

      expect(document.scopeId).to.equal(rawDocument.$scopeId);
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
    });

    it('should create Document with $scope and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $scope: 'test',
        ...data,
      };

      document = new Document(rawDocument);

      expect(document.scope).to.equal(rawDocument.$scope);
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
    });

    it('should create Document with $action and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $action: 'test',
        ...data,
      };

      document = new Document(rawDocument);

      expect(document.action).to.equal(rawDocument.$action);
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
    });

    it('should create Document with $rev and data if present', () => {
      const data = {
        test: 1,
      };

      rawDocument = {
        $rev: 'test',
        ...data,
      };

      document = new Document(rawDocument);

      expect(document.revision).to.equal(rawDocument.$rev);
      expect(Document.prototype.setData).to.have.been.calledOnceWith(data);
    });
  });

  describe('#getId', () => {
    it('should calculate and return ID', () => {
      const idBuffer = Buffer.from('123');
      const id = bs58.encode(idBuffer);

      hashMock.returns(idBuffer);

      const actualId = document.getId();

      expect(hashMock).to.have.been.calledOnceWith(rawDocument.$scope + rawDocument.$scopeId);

      expect(id).to.equal(actualId);
    });

    it('should return already calculated ID', () => {
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

  describe('#setAction', () => {
    it('should set $action', () => {
      const result = document.setAction(Document.ACTIONS.DELETE);

      expect(result).to.equal(document);

      expect(document.action).to.equal(Document.ACTIONS.DELETE);
    });

    it('should throw an error if data is set and the $action is DELETE', () => {
      document.data = {
        test: 1,
      };

      try {
        document.setAction(Document.ACTIONS.DELETE);
      } catch (e) {
        expect(e).to.be.an.instanceOf(DataIsNotAllowedWithActionDeleteError);
        expect(e.getDocument()).to.deep.equal(document);
      }
    });
  });

  describe('#getAction', () => {
    it('should return $action', () => {
      document.action = Document.ACTIONS.DELETE;

      expect(document.getAction()).to.equal(Document.ACTIONS.DELETE);
    });
  });

  describe('#setRevision', () => {
    it('should set $rev', () => {
      const revision = 5;

      const result = document.setRevision(revision);

      expect(result).to.equal(document);

      expect(document.revision).to.equal(revision);
    });
  });

  describe('#getRevision', () => {
    it('should return $rev', () => {
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

    it('should throw an error if $action is already set to DELETE', () => {
      document.setAction(Document.ACTIONS.DELETE);

      const path = 'test[0].$my';
      const value = 2;

      try {
        document.set(path, value);
      } catch (e) {
        expect(e).to.be.an.instanceOf(DataIsNotAllowedWithActionDeleteError);
        expect(e.getDocument()).to.deep.equal(document);
      }
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

  describe('#getMetadata', () => {
    it('should return all meta', () => {
      const meta = new DocumentMetadata({
        userId: 'some string',
      });

      document.metadata = meta;

      expect(document.getMetadata()).to.be.equal(meta);
    });
  });

  describe('#removeMetadata', () => {
    it('should remove all meta', () => {
      const meta = new DocumentMetadata({
        userId: 'some string',
      });

      document.metadata = meta;

      expect(document.getMetadata()).to.deep.equal(meta);

      document.removeMetadata();

      expect(document.getMetadata()).to.be.undefined();
    });
  });

  describe('#toJSON', () => {
    it('should return Document as plain JS object', () => {
      expect(document.toJSON()).to.deep.equal(rawDocument);
    });
  });

  describe('#serialize', () => {
    it('should return serialized Document', () => {
      const serializedDocument = '123';

      encodeMock.returns(serializedDocument);

      const result = document.serialize();

      expect(result).to.equal(serializedDocument);

      expect(encodeMock).to.have.been.calledOnceWith(rawDocument);
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      Document.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return Document hash', () => {
      const serializedDocument = '123';
      const hashedDocument = '456';

      Document.prototype.serialize.returns(serializedDocument);

      hashMock.returns(hashedDocument);

      const result = document.hash();

      expect(result).to.equal(hashedDocument);

      expect(Document.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });
});
