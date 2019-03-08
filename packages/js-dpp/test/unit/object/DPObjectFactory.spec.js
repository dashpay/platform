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

      expect(newDPObject).to.be.an.instanceOf(DPObject);

      expect(newDPObject.getType()).to.equal(rawDPObject.$type);

      expect(newDPObject.get('name')).to.equal(name);

      expect(hashMock).to.have.been.calledOnceWith(dpContract.getId() + userId);
      expect(newDPObject.scope).to.equal(scope);

      expect(generateMock).to.have.been.calledOnce();
      expect(newDPObject.scopeId).to.equal(scopeId);

      expect(newDPObject.getAction()).to.equal(DPObject.DEFAULTS.ACTION);

      expect(newDPObject.getRevision()).to.equal(DPObject.DEFAULTS.REVISION);
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      let error;
      try {
        factory.create(type);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDPObjectTypeError);
      expect(error.getType()).to.equal(type);
      expect(error.getDPContract()).to.equal(dpContract);

      expect(hashMock).to.have.not.been.called();
    });
  });

  describe('createFromObject', () => {
    it('should return new DPContract with data from passed object', () => {
      validateDPObjectMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDPObject);

      expect(result).to.be.an.instanceOf(DPObject);
      expect(result.toJSON()).to.deep.equal(rawDPObject);

      expect(validateDPObjectMock).to.have.been.calledOnceWith(rawDPObject, dpContract);
    });

    it('should return new DPObject without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDPObject, { skipValidation: true });

      expect(result).to.be.an.instanceOf(DPObject);
      expect(result.toJSON()).to.deep.equal(rawDPObject);

      expect(validateDPObjectMock).to.have.not.been.called();
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDPObjectMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDPObject);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDPObjectError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawDPObject()).to.equal(rawDPObject);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.equal(validationError);

      expect(validateDPObjectMock).to.have.been.calledOnceWith(rawDPObject, dpContract);
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

      expect(result).to.equal(dpObject);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDPObject);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDPObject);
    });
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      userId = '123';

      const result = factory.setUserId(userId);

      expect(result).to.equal(factory);
      expect(factory.userId).to.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      const result = factory.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('setDPContract', () => {
    it('should set DP Contract', () => {
      factory.dpContract = null;

      const result = factory.setDPContract(dpContract);

      expect(result).to.equal(factory);
      expect(factory.dpContract).to.equal(dpContract);
    });
  });

  describe('getDPContract', () => {
    it('should return DP Contract', () => {
      const result = factory.getDPContract();

      expect(result).to.equal(dpContract);
    });
  });
});
