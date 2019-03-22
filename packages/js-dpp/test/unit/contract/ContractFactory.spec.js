const rewiremock = require('rewiremock/node');

const getContractFixture = require('../../../lib/test/fixtures/getContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidContractError = require('../../../lib/contract/errors/InvalidContractError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('ContractFactory', () => {
  let ContractFactory;
  let decodeMock;
  let validateContractMock;
  let createContractMock;
  let factory;
  let contract;
  let rawContract;

  beforeEach(function beforeEach() {
    contract = getContractFixture();
    rawContract = contract.toJSON();

    decodeMock = this.sinonSandbox.stub();
    validateContractMock = this.sinonSandbox.stub();
    createContractMock = this.sinonSandbox.stub().returns(contract);

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/contract/ContractFactory');

    ContractFactory = rewiremock.proxy('../../../lib/contract/ContractFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
    });

    factory = new ContractFactory(
      createContractMock,
      validateContractMock,
    );
  });

  describe('create', () => {
    it('should return new Contract with specified name and documents definition', () => {
      const result = factory.create(
        rawContract.name,
        rawContract.documents,
      );

      expect(result).to.equal(contract);

      expect(createContractMock).to.have.been.calledOnceWith({
        name: rawContract.name,
        documents: rawContract.documents,
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new Contract with data from passed object', () => {
      validateContractMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawContract);

      expect(result).to.equal(contract);

      expect(validateContractMock).to.have.been.calledOnceWith(rawContract);

      expect(createContractMock).to.have.been.calledOnceWith(rawContract);
    });

    it('should return new Contract without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawContract, { skipValidation: true });

      expect(result).to.equal(contract);

      expect(validateContractMock).to.have.not.been.called();

      expect(createContractMock).to.have.been.calledOnceWith(rawContract);
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidContractError);
      expect(error.getRawContract()).to.equal(rawContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.equal(validationError);

      expect(validateContractMock).to.have.been.calledOnceWith(rawContract);

      expect(createContractMock).to.have.not.been.called();
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Contract from serialized Contract', () => {
      const serializedContract = contract.serialize();

      decodeMock.returns(rawContract);

      factory.createFromObject.returns(contract);

      const result = factory.createFromSerialized(serializedContract);

      expect(result).to.equal(contract);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawContract);

      expect(decodeMock).to.have.been.calledOnceWith(serializedContract);
    });
  });
});
