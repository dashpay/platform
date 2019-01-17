const rewiremock = require('rewiremock/node');

const DPObject = require('../../../lib/object/DPObject');

const getDPObjectsFixture = require('../../../lib/test/fixtures/getDPObjectsFixture');
const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDPObjectTypeError = require('../../../lib/errors/InvalidDPObjectTypeError');
const InvalidDPObjectError = require('../../../lib/object/errors/InvalidDPObjectError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DPObjectFactory', () => {
  let hashMock;
  let decodeMock;
  let generateMock;
  let validateDPObjectMock;
  let DPObjectFactory;
  let userId;
  let dpContract;
  let dpObject;
  let rawDPObject;
  let factory;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    decodeMock = this.sinonSandbox.stub();
    generateMock = this.sinonSandbox.stub();
    validateDPObjectMock = this.sinonSandbox.stub();

    DPObjectFactory = rewiremock.proxy('../../../lib/object/DPObjectFactory', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': { generate: generateMock },
      '../../../lib/object/DPObject': DPObject,
    });

    ({ userId } = getDPObjectsFixture);
    dpContract = getDPContractFixture();

    [dpObject] = getDPObjectsFixture();
    rawDPObject = dpObject.toJSON();

    factory = new DPObjectFactory(
      userId,
      dpContract,
      validateDPObjectMock,
    );
  });

  describe('create', () => {
    it('should return new DPObject with specified type and data', () => {
      const scope = '123';
      const scopeId = '456';
      const name = 'Cutie';

      hashMock.returns(scope);
      generateMock.returns(scopeId);

      const newDPObject = factory.create(
        rawDPObject.$type,
        { name },
      );

      expect(newDPObject).to.be.instanceOf(DPObject);

      expect(newDPObject.getType()).to.be.equal(rawDPObject.$type);

      expect(newDPObject.get('name')).to.be.equal(name);

      expect(hashMock).to.be.calledOnceWith(dpContract.getId() + userId);
      expect(newDPObject.scope).to.be.equal(scope);

      expect(generateMock).to.be.calledOnce();
      expect(newDPObject.scopeId).to.be.equal(scopeId);

      expect(newDPObject.getAction()).to.be.equal(DPObject.DEFAULTS.ACTION);

      expect(newDPObject.getRevision()).to.be.equal(DPObject.DEFAULTS.REVISION);
    });

    it('should throw error if type is not defined', () => {
      const type = 'wrong';

      let error;
      try {
        factory.create(type);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDPObjectTypeError);
      expect(error.getType()).to.be.equal(type);
      expect(error.getDPContract()).to.be.equal(dpContract);

      expect(hashMock).not.to.be.called();
    });
  });

  describe('createFromObject', () => {
    it('should return new DPContract with data from passed object', () => {
      validateDPObjectMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDPObject);

      expect(result).to.be.instanceOf(DPObject);
      expect(result.toJSON()).to.be.deep.equal(rawDPObject);

      expect(validateDPObjectMock).to.be.calledOnceWith(rawDPObject, dpContract);
    });

    it('should return new DPObject without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDPObject, { skipValidation: true });

      expect(result).to.be.instanceOf(DPObject);
      expect(result.toJSON()).to.be.deep.equal(rawDPObject);

      expect(validateDPObjectMock).not.to.be.called();
    });

    it('should throw error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDPObjectMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDPObject);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDPObjectError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawDPObject()).to.be.equal(rawDPObject);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.be.equal(validationError);

      expect(validateDPObjectMock).to.be.calledOnceWith(rawDPObject, dpContract);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new DPContract from serialized DPContract', () => {
      const serializedDPObject = dpObject.serialize();

      decodeMock.returns(rawDPObject);

      factory.createFromObject.returns(dpObject);

      const result = factory.createFromSerialized(serializedDPObject);

      expect(result).to.be.equal(dpObject);

      expect(factory.createFromObject).to.be.calledOnceWith(rawDPObject);

      expect(decodeMock).to.be.calledOnceWith(serializedDPObject);
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

  describe('setDPContract', () => {
    it('should set DP Contract', () => {
      factory.dpContract = null;

      const result = factory.setDPContract(dpContract);

      expect(result).to.be.equal(factory);
      expect(factory.dpContract).to.be.equal(dpContract);
    });
  });

  describe('getDPContract', () => {
    it('should return DP Contract', () => {
      const result = factory.getDPContract();

      expect(result).to.be.equal(dpContract);
    });
  });
});
