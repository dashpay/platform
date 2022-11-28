const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const DataContractCreateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const InvalidDataContractError = require('@dashevo/dpp/lib/dataContract/errors/InvalidDataContractError');
const SerializedObjectParsingError = require('@dashevo/dpp/lib/errors/consensus/basic/decode/SerializedObjectParsingError');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');

const entropyGenerator = require('@dashevo/dpp/lib/util/entropyGenerator');

const { default: loadWasmDpp } = require('../../../dist');

describe('DataContractFactory', () => {
  let DataContractFactory;
  let DataContractValidator;

  let factory;
  let dataContract;
  let rawDataContract;
  let dataContractValidator;

  before(async () => {
    ({
      DataContractFactory, DataContractValidator,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    dataContract = getDataContractFixture();

    // For some reason fixture has empty $defs which violates meta schema
    delete dataContract.$defs;

    rawDataContract = dataContract.toObject();

    dataContractValidator = new DataContractValidator();

    factory = new DataContractFactory(
      1,
      dataContractValidator,
    );
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      const result = factory.create(
        dataContract.ownerId.toBuffer(),
        rawDataContract.documents,
      ).toObject();

      expect(result).excluding("$id").to.deep.equal(rawDataContract);
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      const result = await factory.createFromObject(rawDataContract);
      expect(result.toObject()).excluding('entropy').to.deep.equal(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      let alteredContract = dataContract.toObject();
      alteredContract.$defs = {}; // Empty defs are bad!

      const resultSkipValidation = await factory.createFromObject(alteredContract, true);

      expect(resultSkipValidation.toObject()).excluding('entropy').to.deep.equal(alteredContract);
    });

    it('should throw an error if passed object is not valid', async () => {
      let alteredContract = dataContract.toObject();
      alteredContract.$defs = {}; // Empty defs are bad!

      let error;
      try {
        await factory.createFromObject(alteredContract);
      } catch (e) {
        error = e;
      }

      // TODO
      // expect(error.getRawDataContract()).to.equal(alteredContract);
      // expect(error.getErrors()).to.have.length(1);

      // const [consensusError] = error.getErrors();

      // expect(consensusError).to.equal(validationError);
    });
  });

  describe('createFromBuffer', () => {
    let serializedDataContract;

    beforeEach(() => {
      serializedDataContract = dataContract.toBuffer();
    });

    it('should return new Data Contract from serialized contract', async () => {
      const result = await factory.createFromBuffer(serializedDataContract);

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });

    // it('should throw InvalidDataContractError if the decoding fails with consensus error', async () => {
    //   const parsingError = new SerializedObjectParsingError(
    //     serializedDataContract,
    //     new Error(),
    //   );

    //   decodeProtocolEntityMock.throws(parsingError);

    //   try {
    //     await factory.createFromBuffer(serializedDataContract);

    //     expect.fail('should throw InvalidDataContractError');
    //   } catch (e) {
    //     expect(e).to.be.an.instanceOf(InvalidDataContractError);

    //     const [innerError] = e.getErrors();
    //     expect(innerError).to.equal(parsingError);
    //   }
    // });

    // it('should throw an error if decoding fails with any other error', async () => {
    //   const parsingError = new Error('Something failed during parsing');

    //   decodeProtocolEntityMock.throws(parsingError);

    //   try {
    //     await factory.createFromBuffer(serializedDataContract);

    //     expect.fail('should throw an error');
    //   } catch (e) {
    //     expect(e).to.equal(parsingError);
    //   }
    // });
  });

  // describe('createDataContractCreateTransition', () => {
  //   it('should return new DataContractCreateTransition with passed DataContract', () => {
  //     const result = factory.createDataContractCreateTransition(dataContract);

  //     expect(result).to.be.an.instanceOf(DataContractCreateTransition);

  //     expect(result.getProtocolVersion()).to.equal(protocolVersion.latestVersion);
  //     expect(result.getEntropy()).to.deep.equal(dataContract.getEntropy());
  //     expect(result.getDataContract().toObject()).to.deep.equal(dataContract.toObject());
  //   });
  // });
});
