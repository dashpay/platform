const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const DataContract = require('../../../lib/dataContract/DataContract');
const protocolVersion = require('../../../lib/protocolVersion');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDataContractError = require('../../../lib/dataContract/errors/InvalidDataContractError');
const ConsensusError = require('../../../lib/errors/consensus/ConsensusError');
const SerializedObjectParsingError = require('../../../lib/errors/consensus/basic/decode/SerializedObjectParsingError');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');

describe('DataContractFactory', () => {
  let DataContractFactory;
  let decodeProtocolEntityMock;
  let validateDataContractMock;
  let DataContractMock;
  let factory;
  let dataContract;
  let rawDataContract;
  let generateEntropyMock;
  let dppMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    decodeProtocolEntityMock = this.sinonSandbox.stub();
    validateDataContractMock = this.sinonSandbox.stub();

    DataContractMock = this.sinonSandbox.stub().returns(dataContract);
    DataContractMock.DEFAULTS = DataContract.DEFAULTS;

    generateEntropyMock = this.sinonSandbox.stub();

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/dataContract/DataContractFactory');

    DataContractFactory = rewiremock.proxy('../../../lib/dataContract/DataContractFactory', {
      '../../../lib/util/generateEntropy': generateEntropyMock,
      '../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition': DataContractCreateTransition,
      '../../../lib/dataContract/DataContract': DataContractMock,
    });

    dppMock = createDPPMock();

    factory = new DataContractFactory(
      dppMock,
      validateDataContractMock,
      decodeProtocolEntityMock,
    );
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      generateEntropyMock.returns(dataContract.getEntropy());
      const result = factory.create(
        dataContract.ownerId.toBuffer(),
        rawDataContract.documents,
      );

      expect(result).to.equal(dataContract);

      expect(DataContractMock).to.have.been.calledOnceWith({
        protocolVersion: protocolVersion.latestVersion,
        $schema: DataContract.DEFAULTS.SCHEMA,
        $id: dataContract.id.toBuffer(),
        ownerId: dataContract.ownerId.toBuffer(),
        documents: rawDataContract.documents,
        $defs: {},
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDataContractMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawDataContract);

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);

      expect(DataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawDataContract, { skipValidation: true });

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.not.been.called();

      expect(DataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should throw an error if passed object is not valid', async () => {
      const validationError = new ConsensusError('test');

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

      expect(DataContractMock).to.have.not.been.called();
    });
  });

  describe('createFromBuffer', () => {
    let serializedDataContract;

    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');

      serializedDataContract = dataContract.toBuffer();
    });

    it('should return new Data Contract from serialized contract', async () => {
      decodeProtocolEntityMock.returns([rawDataContract.protocolVersion, rawDataContract]);

      factory.createFromObject.returns(dataContract);

      const result = await factory.createFromBuffer(serializedDataContract);

      expect(result).to.equal(dataContract);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDataContract);

      expect(decodeProtocolEntityMock).to.have.been.calledOnceWith(
        serializedDataContract,
        dppMock.getProtocolVersion(),
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
