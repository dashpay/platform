const rewiremock = require('rewiremock/node');

describe('DPObject', () => {
  let lodashGetMock;
  let lodashSetMock;
  let hashMock;
  let encodeMock;
  let DPObject;
  let rawDPObject;
  let dpObject;

  beforeEach(function beforeEach() {
    lodashGetMock = this.sinonSandbox.stub();
    lodashSetMock = this.sinonSandbox.stub();
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DPObject = rewiremock.proxy('../../../lib/object/DPObject', {
      '../../../node_modules/lodash.get': lodashGetMock,
      '../../../node_modules/lodash.set': lodashSetMock,
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    rawDPObject = {
      $type: 'test',
      $scope: 'a832e4145650bfe8462e768e9c4a9a0d3a0bb7dcd9b3e50c61c73ac9d2e14068',
      $scopeId: 'ydhM7GjG4QUbcuXpZDVoi7TTn7LL8Rhgzh',
      $action: DPObject.DEFAULTS.ACTION,
      $rev: DPObject.DEFAULTS.REVISION,
    };

    dpObject = new DPObject(rawDPObject);
  });

  describe('constructor', () => {
    beforeEach(function beforeEach() {
      DPObject.prototype.setData = this.sinonSandbox.stub();
    });

    it('should create DP Object with $type and data if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $type: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.type).to.be.equal(rawDPObject.$type);
      expect(DPObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DP Object with $scopeId and data  if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $scopeId: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.scopeId).to.be.equal(rawDPObject.$scopeId);
      expect(DPObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DP Object with $scope and data if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $scope: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.scope).to.be.equal(rawDPObject.$scope);
      expect(DPObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DP Object with $action and data if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $action: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.action).to.be.equal(rawDPObject.$action);
      expect(DPObject.prototype.setData).to.be.calledOnceWith(data);
    });

    it('should create DP Object with $rev and data if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $rev: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.revision).to.be.equal(rawDPObject.$rev);
      expect(DPObject.prototype.setData).to.be.calledOnceWith(data);
    });
  });

  describe('#getId', () => {
    it('should calculate and return ID', () => {
      const id = '123';

      hashMock.returns(id);

      const actualId = dpObject.getId();

      expect(hashMock).to.be.calledOnceWith(rawDPObject.$scope + rawDPObject.$scopeId);

      expect(id).to.be.equal(actualId);
    });

    it('should return already calculated ID', () => {
      const id = '123';

      dpObject.id = id;

      const actualId = dpObject.getId();

      expect(hashMock).not.to.be.called();

      expect(id).to.be.equal(actualId);
    });
  });

  describe('#getType', () => {
    it('should return $type', () => {
      expect(dpObject.getType()).to.be.equal(rawDPObject.$type);
    });
  });

  describe('#setAction', () => {
    it('should set $action', () => {
      const result = dpObject.setAction(DPObject.ACTIONS.DELETE);

      expect(result).to.be.equal(dpObject);

      expect(dpObject.action).to.be.equal(DPObject.ACTIONS.DELETE);
    });
  });

  describe('#getAction', () => {
    it('should return $action', () => {
      dpObject.action = DPObject.ACTIONS.DELETE;

      expect(dpObject.getAction()).to.be.equal(DPObject.ACTIONS.DELETE);
    });
  });

  describe('#setRevision', () => {
    it('should set $rev', () => {
      const revision = 5;

      const result = dpObject.setRevision(revision);

      expect(result).to.be.equal(dpObject);

      expect(dpObject.revision).to.be.equal(revision);
    });
  });

  describe('#getRevision', () => {
    it('should return $rev', () => {
      const revision = 5;

      dpObject.revision = revision;

      expect(dpObject.getRevision()).to.be.equal(revision);
    });
  });

  describe('#setData', () => {
    beforeEach(function beforeEach() {
      DPObject.prototype.set = this.sinonSandbox.stub();
    });

    it('should call set for each object property', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      const result = dpObject.setData(data);

      expect(result).to.be.equal(dpObject);

      expect(DPObject.prototype.set).to.be.calledTwice();

      expect(DPObject.prototype.set).to.be.calledWith('test1', 1);
      expect(DPObject.prototype.set).to.be.calledWith('test2', 2);
    });
  });

  describe('#getData', () => {
    it('should return all data', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      dpObject.data = data;

      expect(dpObject.getData()).to.be.equal(data);
    });
  });

  describe('#set', () => {
    it('should set value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      const result = dpObject.set(path, value);

      expect(result).to.be.equal(dpObject);

      expect(lodashSetMock).to.be.calledOnceWith(dpObject.data, path, value);
    });
  });

  describe('#get', () => {
    it('should return value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      lodashGetMock.returns(value);

      const result = dpObject.get(path);

      expect(result).to.be.equal(value);

      expect(lodashGetMock).to.be.calledOnceWith(dpObject.data, path);
    });
  });

  describe('#toJSON', () => {
    it('should return DPObject as plain JS object', () => {
      expect(dpObject.toJSON()).to.be.deep.equal(rawDPObject);
    });
  });

  describe('#serialize', () => {
    it('should return serialized DPObject', () => {
      const serializedObject = '123';

      encodeMock.returns(serializedObject);

      const result = dpObject.serialize();

      expect(result).to.be.equal(serializedObject);

      expect(encodeMock).to.be.calledOnceWith(rawDPObject);
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DPObject.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return DPObject hash', () => {
      const serializedObject = '123';
      const hashedObject = '456';

      DPObject.prototype.serialize.returns(serializedObject);

      hashMock.returns(hashedObject);

      const result = dpObject.hash();

      expect(result).to.be.equal(hashedObject);

      expect(DPObject.prototype.serialize).to.be.calledOnce();

      expect(hashMock).to.be.calledOnceWith(serializedObject);
    });
  });
});
