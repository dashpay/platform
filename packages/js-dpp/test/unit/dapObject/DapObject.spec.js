const rewiremock = require('rewiremock/node');

describe('DapObject', () => {
  let lodashGetMock;
  let lodashSetMock;
  let hashMock;
  let encodeMock;
  let DapObject;
  let rawDapObject;
  let dapObject;

  beforeEach(function beforeEach() {
    lodashGetMock = this.sinonSandbox.stub();
    lodashSetMock = this.sinonSandbox.stub();
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DapObject = rewiremock.proxy('../../../lib/dapObject/DapObject', {
      '../../../node_modules/lodash.get': lodashGetMock,
      '../../../node_modules/lodash.set': lodashSetMock,
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    rawDapObject = {
      $type: 'test',
      $scope: 'a832e4145650bfe8462e768e9c4a9a0d3a0bb7dcd9b3e50c61c73ac9d2e14068',
      $scopeId: 'ydhM7GjG4QUbcuXpZDVoi7TTn7LL8Rhgzh',
      $action: DapObject.DEFAULTS.ACTION,
      $rev: DapObject.DEFAULTS.REVISION,
    };

    dapObject = new DapObject(rawDapObject);
  });

  describe('constructor', () => {
    beforeEach(function beforeEach() {
      DapObject.prototype.setData = this.sinonSandbox.stub();
    });

    it('should create DAP Object with $type and data if present', () => {
      const data = {
        test: 1,
      };

      rawDapObject = {
        $type: 'test',
        ...data,
      };

      dapObject = new DapObject(rawDapObject);

      expect(dapObject.type).to.be.equal(rawDapObject.$type);
      expect(DapObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DAP Object with $scopeId and data  if present', () => {
      const data = {
        test: 1,
      };

      rawDapObject = {
        $scopeId: 'test',
        ...data,
      };

      dapObject = new DapObject(rawDapObject);

      expect(dapObject.scopeId).to.be.equal(rawDapObject.$scopeId);
      expect(DapObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DAP Object with $scope and data if present', () => {
      const data = {
        test: 1,
      };

      rawDapObject = {
        $scope: 'test',
        ...data,
      };

      dapObject = new DapObject(rawDapObject);

      expect(dapObject.scope).to.be.equal(rawDapObject.$scope);
      expect(DapObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DAP Object with $action and data if present', () => {
      const data = {
        test: 1,
      };

      rawDapObject = {
        $action: 'test',
        ...data,
      };

      dapObject = new DapObject(rawDapObject);

      expect(dapObject.action).to.be.equal(rawDapObject.$action);
      expect(DapObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DAP Object with $rev and data if present', () => {
      const data = {
        test: 1,
      };

      rawDapObject = {
        $rev: 'test',
        ...data,
      };

      dapObject = new DapObject(rawDapObject);

      expect(dapObject.revision).to.be.equal(rawDapObject.$rev);
      expect(DapObject.prototype.setData).to.be.calledOnceWith(data);
    });
  });

  describe('#getId', () => {
    it('should calculate and return ID', () => {
      const id = '123';

      hashMock.returns(id);

      const actualId = dapObject.getId();

      expect(hashMock).to.be.calledOnceWith(rawDapObject.$scope + rawDapObject.$scopeId);

      expect(id).to.be.equal(actualId);
    });

    it('should return already calculated ID', () => {
      const id = '123';

      dapObject.id = id;

      const actualId = dapObject.getId();

      expect(hashMock).not.to.be.called();

      expect(id).to.be.equal(actualId);
    });
  });

  describe('#getType', () => {
    it('should return $type', () => {
      expect(dapObject.getType()).to.be.equal(rawDapObject.$type);
    });
  });

  describe('#setAction', () => {
    it('should set $action', () => {
      const result = dapObject.setAction(DapObject.ACTIONS.DELETE);

      expect(result).to.be.equal(dapObject);

      expect(dapObject.action).to.be.equal(DapObject.ACTIONS.DELETE);
    });
  });

  describe('#getAction', () => {
    it('should return $action', () => {
      dapObject.action = DapObject.ACTIONS.DELETE;

      expect(dapObject.getAction()).to.be.equal(DapObject.ACTIONS.DELETE);
    });
  });

  describe('#setRevision', () => {
    it('should set $rev', () => {
      const revision = 5;

      const result = dapObject.setRevision(revision);

      expect(result).to.be.equal(dapObject);

      expect(dapObject.revision).to.be.equal(revision);
    });
  });

  describe('#getRevision', () => {
    it('should return $rev', () => {
      const revision = 5;

      dapObject.revision = revision;

      expect(dapObject.getRevision()).to.be.equal(revision);
    });
  });

  describe('#setData', () => {
    beforeEach(function beforeEach() {
      DapObject.prototype.set = this.sinonSandbox.stub();
    });

    it('should call set for each object property', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      const result = dapObject.setData(data);

      expect(result).to.be.equal(dapObject);

      expect(DapObject.prototype.set).to.be.calledTwice();

      expect(DapObject.prototype.set).to.be.calledWith('test1', 1);
      expect(DapObject.prototype.set).to.be.calledWith('test2', 2);
    });
  });

  describe('#getData', () => {
    it('should return all data', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      dapObject.data = data;

      expect(dapObject.getData()).to.be.equal(data);
    });
  });

  describe('#set', () => {
    it('should set value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      const result = dapObject.set(path, value);

      expect(result).to.be.equal(dapObject);

      expect(lodashSetMock).to.be.calledOnceWith(dapObject.data, path, value);
    });
  });

  describe('#get', () => {
    it('should return value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      lodashGetMock.returns(value);

      const result = dapObject.get(path);

      expect(result).to.be.equal(value);

      expect(lodashGetMock).to.be.calledOnceWith(dapObject.data, path);
    });
  });

  describe('#toJSON', () => {
    it('should return Dap Object as plain JS object', () => {
      expect(dapObject.toJSON()).to.be.deep.equal(rawDapObject);
    });
  });

  describe('#serialize', () => {
    it('should return serialized Dap Object', () => {
      const serializedObject = '123';

      encodeMock.returns(serializedObject);

      const result = dapObject.serialize();

      expect(result).to.be.equal(serializedObject);

      expect(encodeMock).to.be.calledOnceWith(rawDapObject);
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DapObject.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return Dap Object hash', () => {
      const serializedObject = '123';
      const hashedObject = '456';

      DapObject.prototype.serialize.returns(serializedObject);

      hashMock.returns(hashedObject);

      const result = dapObject.hash();

      expect(result).to.be.equal(hashedObject);

      expect(DapObject.prototype.serialize).to.be.calledOnce();

      expect(hashMock).to.be.calledOnceWith(serializedObject);
    });
  });
});
