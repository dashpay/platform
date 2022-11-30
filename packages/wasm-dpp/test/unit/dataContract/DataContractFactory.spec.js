const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../dist');

describe('DataContractFactory', () => {
  let DataContractFactory;
  let DataContractValidator;
  let DataContract;

  let factory;
  let jsDataContract;
  let rawDataContract;
  let dataContractValidator;

  before(async () => {
    ({
      DataContractFactory, DataContractValidator, DataContract, InvalidDataContractError,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    jsDataContract = getDataContractFixture();

    // For some reason fixture has empty $defs which violates meta schema
    delete jsDataContract.$defs;

    rawDataContract = jsDataContract.toObject();

    dataContractValidator = new DataContractValidator();

    factory = new DataContractFactory(
      1,
      dataContractValidator,
    );
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      const result = factory.create(
        jsDataContract.ownerId.toBuffer(),
        rawDataContract.documents,
      ).toObject();

      expect(result).excluding('$id').to.deep.equal(rawDataContract);
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      const result = await factory.createFromObject(rawDataContract);
      expect(result.toObject()).excluding('entropy').to.deep.equal(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      const alteredContract = jsDataContract.toObject();
      alteredContract.$defs = {}; // Empty defs are bad!

      const resultSkipValidation = await factory.createFromObject(alteredContract, true);

      expect(resultSkipValidation.toObject()).excluding('entropy').to.deep.equal(alteredContract);
    });

    it('should throw an error if passed object is not valid', async () => {
      const alteredContract = jsDataContract.toObject();
      alteredContract.$defs = {}; // Empty defs are bad!

      try {
        await factory.createFromObject(alteredContract);
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDataContractError);
        expect(e.getRawDataContract()).to.deep.equal(alteredContract);
        expect(e.getErrors()).to.have.length(1);
        // TODO JsonSchemaError binding type required
        // const [consensusError] = error.getErrors();
      }
    });
  });

  describe('createFromBuffer', () => {
    let serializedDataContract;

    beforeEach(() => {
      serializedDataContract = jsDataContract.toBuffer();
    });

    it('should return new Data Contract from serialized contract', async () => {
      const result = await factory.createFromBuffer(serializedDataContract);

      expect(result.toObject()).to.deep.equal(jsDataContract.toObject());
    });

    it('should throw InvalidDataContractError if the decoding fails with consensus error', async () => {
      try {
        // Casually break serialized data contract:
        serializedDataContract[5] += 1337;
        await factory.createFromBuffer(serializedDataContract);
        expect.fail('should throw InvalidDataContractError');
      } catch (e) {
        // TODO SerializedObjectParsingError binding type required
        //expect(e).to.be.an.instanceOf(SerializedObjectPasingError);
        expect(e).to.match(/Parsing of serialized object failed/);
      }
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should return new DataContractCreateTransition with passed DataContract', async () => {
      // Create wasm version of DataContract
      const dataContract = new DataContract(rawDataContract);
      dataContract.setEntropy(jsDataContract.getEntropy());

      const result = await factory.createDataContractCreateTransition(dataContract);

      expect(result.getProtocolVersion()).to.equal(protocolVersion.latestVersion);
      expect(result.getEntropy()).to.deep.equal(jsDataContract.getEntropy());
      expect(result.getDataContract().toObject()).to.deep.equal(jsDataContract.toObject());
    });
  });
});
