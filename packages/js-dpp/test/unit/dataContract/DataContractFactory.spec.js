const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const protocolVersion = require('../../../lib/version/protocolVersion');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDataContractError = require('../../../lib/dataContract/errors/InvalidDataContractError');
const SerializedObjectParsingError = require('../../../lib/errors/consensus/basic/decode/SerializedObjectParsingError');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');
const SomeConsensusError = require('../../../lib/test/mocks/SomeConsensusError');

const DataContractFactory = require('../../../lib/dataContract/DataContractFactory');
const entropyGenerator = require('../../../lib/util/entropyGenerator');

describe('DataContractFactory', () => {
  let decodeProtocolEntityMock;
  let validateDataContractMock;
  let factory;
  let dataContract;
  let rawDataContract;
  let generateEntropyMock;
  let dppMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    dppMock = createDPPMock();

    rawDataContract = dataContract.toObject();

    decodeProtocolEntityMock = this.sinonSandbox.stub();
    validateDataContractMock = this.sinonSandbox.stub();
    generateEntropyMock = this.sinonSandbox.stub(entropyGenerator, 'generate');

    factory = new DataContractFactory(
      dppMock,
      validateDataContractMock,
      decodeProtocolEntityMock,
    );
  });

  afterEach(() => {
    generateEntropyMock.restore();
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      generateEntropyMock.returns(dataContract.getEntropy());
      const result = factory.create(
        dataContract.ownerId.toBuffer(),
        rawDataContract.documents,
      );

      expect(result).excluding('$defs').to.deep.equal(dataContract);
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDataContractMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawDataContract);

      expect(result).excluding('entropy').to.deep.equal(dataContract);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawDataContract, { skipValidation: true });

      expect(result).excluding('entropy').to.deep.equal(dataContract);

      expect(validateDataContractMock).to.have.not.been.called();
    });

    it('should throw an error if passed object is not valid', async () => {
      const validationError = new SomeConsensusError('test');

      validateDataContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        await factory.createFromObject(rawDataContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDataContractError);
      expect(error.getRawDataContract()).to.equal(rawDataContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.equal(validationError);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });
  });

  describe('createFromBuffer', () => {
    let serializedDataContract;

    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');

      serializedDataContract = dataContract.toBuffer();
    });

    afterEach(() => {
      factory.createFromObject.restore();
    });

    it('should return new Data Contract from serialized contract', async () => {
      decodeProtocolEntityMock.returns([rawDataContract.protocolVersion, rawDataContract]);

      factory.createFromObject.returns(dataContract);

      const result = await factory.createFromBuffer(serializedDataContract);

      expect(result).to.equal(dataContract);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDataContract);

      expect(decodeProtocolEntityMock).to.have.been.calledOnceWithExactly(
        serializedDataContract,
      );
    });

    it('should throw InvalidDataContractError if the decoding fails with consensus error', async () => {
      const parsingError = new SerializedObjectParsingError(
        serializedDataContract,
        new Error(),
      );

      decodeProtocolEntityMock.throws(parsingError);

      try {
        await factory.createFromBuffer(serializedDataContract);

        expect.fail('should throw InvalidDataContractError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDataContractError);

        const [innerError] = e.getErrors();
        expect(innerError).to.equal(parsingError);
      }
    });

    it('should throw an error if decoding fails with any other error', async () => {
      const parsingError = new Error('Something failed during parsing');

      decodeProtocolEntityMock.throws(parsingError);

      try {
        await factory.createFromBuffer(serializedDataContract);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.equal(parsingError);
      }
    });
  });

  describe('createStateTransition', () => {
    it('should return new DataContractCreateTransition with passed DataContract', () => {
      const result = factory.createStateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.getProtocolVersion()).to.equal(protocolVersion.latestVersion);
      expect(result.getEntropy()).to.deep.equal(dataContract.getEntropy());
      expect(result.getDataContract().toObject()).to.deep.equal(dataContract.toObject());
    });
  });
});
