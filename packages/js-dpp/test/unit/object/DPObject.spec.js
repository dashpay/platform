const rewiremock = require('rewiremock/node');

const DataIsNotAllowedWithActionDeleteError = require('../../../lib/object/errors/DataIsNotAllowedWithActionDeleteError');

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

      expect(dpObject.type).to.equal(rawDPObject.$type);
      expect(DPObject.prototype.setData).to.have.been.calledOnceWith(data);
    });

    it('should create DP Object with $scopeId and data if present', () => {
      const data = {
        test: 1,
      };

      rawDPObject = {
        $scopeId: 'test',
        ...data,
      };

      dpObject = new DPObject(rawDPObject);

      expect(dpObject.scopeId).to.equal(rawDPObject.$scopeId);
      expect(DPObject.prototype.setData).to.have.been.calledOnceWith(data);
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

      expect(dpObject.scope).to.equal(rawDPObject.$scope);
      expect(DPObject.prototype.setData).to.have.been.calledOnceWith(data);
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

      expect(dpObject.action).to.equal(rawDPObject.$action);
      expect(DPObject.prototype.setData).to.have.been.calledOnceWith(data);
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

      expect(dpObject.revision).to.equal(rawDPObject.$rev);
      expect(DPObject.prototype.setData).to.have.been.calledOnceWith(data);
    });
  });

  describe('#getId', () => {
    it('should calculate and return ID', () => {
      const id = '123';

      hashMock.returns(id);

      const actualId = dpObject.getId();

      expect(hashMock).to.have.been.calledOnceWith(rawDPObject.$scope + rawDPObject.$scopeId);

      expect(id).to.equal(actualId);
    });

    it('should return already calculated ID', () => {
      const id = '123';

      dpObject.id = id;

      const actualId = dpObject.getId();

      expect(hashMock).to.have.not.been.called();

      expect(id).to.equal(actualId);
    });
  });

  describe('#getType', () => {
    it('should return $type', () => {
      expect(dpObject.getType()).to.equal(rawDPObject.$type);
    });
  });

  describe('#setAction', () => {
    it('should set $action', () => {
      const result = dpObject.setAction(DPObject.ACTIONS.DELETE);

      expect(result).to.equal(dpObject);

      expect(dpObject.action).to.equal(DPObject.ACTIONS.DELETE);
    });

    it('should throw an error if data is set and the $action is DELETE', () => {
      dpObject.data = {
        test: 1,
      };

      try {
        dpObject.setAction(DPObject.ACTIONS.DELETE);
      } catch (e) {
        expect(e).to.be.an.instanceOf(DataIsNotAllowedWithActionDeleteError);
        expect(e.getDPObject()).to.deep.equal(dpObject);
      }
    });
  });

  describe('#getAction', () => {
    it('should return $action', () => {
      dpObject.action = DPObject.ACTIONS.DELETE;

      expect(dpObject.getAction()).to.equal(DPObject.ACTIONS.DELETE);
    });
  });

  describe('#setRevision', () => {
    it('should set $rev', () => {
      const revision = 5;

      const result = dpObject.setRevision(revision);

      expect(result).to.equal(dpObject);

      expect(dpObject.revision).to.equal(revision);
    });
  });

  describe('#getRevision', () => {
    it('should return $rev', () => {
      const revision = 5;

      dpObject.revision = revision;

      expect(dpObject.getRevision()).to.equal(revision);
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

      expect(result).to.equal(dpObject);

      expect(DPObject.prototype.set).to.have.been.calledTwice();

      expect(DPObject.prototype.set).to.have.been.calledWith('test1', 1);
      expect(DPObject.prototype.set).to.have.been.calledWith('test2', 2);
    });
  });

  describe('#getData', () => {
    it('should return all data', () => {
      const data = {
        test1: 1,
        test2: 2,
      };

      dpObject.data = data;

      expect(dpObject.getData()).to.equal(data);
    });
  });

  describe('#set', () => {
    it('should set value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      const result = dpObject.set(path, value);

      expect(result).to.equal(dpObject);

      expect(lodashSetMock).to.have.been.calledOnceWith(dpObject.data, path, value);
    });

    it('should throw an error if $action is already set to DELETE', () => {
      dpObject.setAction(DPObject.ACTIONS.DELETE);

      const path = 'test[0].$my';
      const value = 2;

      try {
        dpObject.set(path, value);
      } catch (e) {
        expect(e).to.be.an.instanceOf(DataIsNotAllowedWithActionDeleteError);
        expect(e.getDPObject()).to.deep.equal(dpObject);
      }
    });
  });

  describe('#get', () => {
    it('should return value for specified property name', () => {
      const path = 'test[0].$my';
      const value = 2;

      lodashGetMock.returns(value);

      const result = dpObject.get(path);

      expect(result).to.equal(value);

      expect(lodashGetMock).to.have.been.calledOnceWith(dpObject.data, path);
    });
  });

  describe('#toJSON', () => {
    it('should return DPObject as plain JS object', () => {
      expect(dpObject.toJSON()).to.deep.equal(rawDPObject);
    });
  });

  describe('#serialize', () => {
    it('should return serialized DPObject', () => {
      const serializedObject = '123';

      encodeMock.returns(serializedObject);

      const result = dpObject.serialize();

      expect(result).to.equal(serializedObject);

      expect(encodeMock).to.have.been.calledOnceWith(rawDPObject);
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

      expect(result).to.equal(hashedObject);

      expect(DPObject.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedObject);
    });
  });
});
