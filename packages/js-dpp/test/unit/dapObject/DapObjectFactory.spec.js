const rewiremock = require('rewiremock/node');

const DapObject = require('../../../lib/dapObject/DapObject');

const getDapObjectsFixture = require('../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDapObjectTypeError = require('../../../lib/errors/InvalidDapObjectTypeError');
const InvalidDapObjectError = require('../../../lib/dapObject/errors/InvalidDapObjectError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DapObjectFactory', () => {
  let hashMock;
  let decodeMock;
  let generateMock;
  let validateDapObjectMock;
  let DapObjectFactory;
  let userId;
  let dapContract;
  let dapObject;
  let rawDapObject;
  let factory;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    decodeMock = this.sinonSandbox.stub();
    generateMock = this.sinonSandbox.stub();
    validateDapObjectMock = this.sinonSandbox.stub();

    DapObjectFactory = rewiremock.proxy('../../../lib/dapObject/DapObjectFactory', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': { generate: generateMock },
      '../../../lib/dapObject/DapObject': DapObject,
    });

    ({ userId } = getDapObjectsFixture);
    dapContract = getDapContractFixture();

    [dapObject] = getDapObjectsFixture();
    rawDapObject = dapObject.toJSON();

    factory = new DapObjectFactory(
      userId,
      dapContract,
      validateDapObjectMock,
    );
  });

  describe('create', () => {
    it('should return new DapObject with specified type and data', () => {
      const scope = '123';
      const scopeId = '456';
      const name = 'Cutie';

      hashMock.returns(scope);
      generateMock.returns(scopeId);

      const newDapObject = factory.create(
        rawDapObject.$type,
        { name },
      );

      expect(newDapObject).to.be.instanceOf(DapObject);

      expect(newDapObject.getType()).to.be.equal(rawDapObject.$type);

      expect(newDapObject.get('name')).to.be.equal(name);

      expect(hashMock).to.be.calledOnceWith(dapContract.getId() + userId);
      expect(newDapObject.scope).to.be.equal(scope);

      expect(generateMock).to.be.calledOnce();
      expect(newDapObject.scopeId).to.be.equal(scopeId);

      expect(newDapObject.getAction()).to.be.equal(DapObject.DEFAULTS.ACTION);

      expect(newDapObject.getRevision()).to.be.equal(DapObject.DEFAULTS.REVISION);
    });

    it('should throw error if type is not defined', () => {
      const type = 'wrong';

      let error;
      try {
        factory.create(type);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
      expect(error.getType()).to.be.equal(type);
      expect(error.getDapContract()).to.be.equal(dapContract);

      expect(hashMock).not.to.be.called();
    });
  });

  describe('createFromObject', () => {
    it('should return new DapContract with data from passed object', () => {
      validateDapObjectMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDapObject);

      expect(result).to.be.instanceOf(DapObject);
      expect(result.toJSON()).to.be.deep.equal(rawDapObject);

      expect(validateDapObjectMock).to.be.calledOnceWith(rawDapObject, dapContract);
    });

    it('should throw error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDapObjectMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDapObject);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDapObjectError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawDapObject()).to.be.equal(rawDapObject);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.be.equal(validationError);

      expect(validateDapObjectMock).to.be.calledOnceWith(rawDapObject, dapContract);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new DapContract from serialized DapContract', () => {
      const serializedDapObject = dapObject.serialize();

      decodeMock.returns(rawDapObject);

      factory.createFromObject.returns(dapObject);

      const result = factory.createFromSerialized(serializedDapObject);

      expect(result).to.be.equal(dapObject);

      expect(factory.createFromObject).to.be.calledOnceWith(rawDapObject);

      expect(decodeMock).to.be.calledOnceWith(serializedDapObject);
    });
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      userId = '123';

      const result = factory.setUserId(userId);

      expect(result).to.be.equal(factory);
      expect(factory.userId).to.be.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      const result = factory.getUserId();

      expect(result).to.be.equal(userId);
    });
  });

  describe('setDapContract', () => {
    it('should set DAP Contract', () => {
      factory.dapContract = null;

      const result = factory.setDapContract(dapContract);

      expect(result).to.be.equal(factory);
      expect(factory.dapContract).to.be.equal(dapContract);
    });
  });

  describe('getDapContract', () => {
    it('should return DAP Contract', () => {
      const result = factory.getDapContract();

      expect(result).to.be.equal(dapContract);
    });
  });
});
