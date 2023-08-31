const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp, UnsupportedProtocolVersionError } = require('../../..');
const { getLatestProtocolVersion } = require('../../..');

describe.skip('DataContractFactory', () => {
  let DataContractFactory;
  let DataContractValidator;
  let DataContract;
  let InvalidDataContractError;
  let JsonSchemaError;

  let factory;
  let dataContract;
  let rawDataContract;
  let dataContractValidator;

  before(async () => {
    ({
      DataContractFactory,
      DataContractValidator,
      DataContract,
      InvalidDataContractError,
      JsonSchemaError,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();
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
        dataContract.getOwnerId(),
        rawDataContract.documents,
      ).toObject();

      expect(result).excluding('$id').to.deep.equal(rawDataContract);
    });

    it('should pass config', () => {
      const config = {
        canBeDeleted: false,
        readonly: false,
        keepsHistory: true,
        documentsKeepHistoryContractDefault: false,
        documentsMutableContractDefault: true,
      };
      const contract = factory.create(
        dataContract.getOwnerId(),
        rawDataContract.documents,
        config,
      );

      const result = contract.toObject();

      expect(contract.getConfig()).to.be.deep.equal(config);
      expect(result).excluding('$id').to.deep.equal(rawDataContract);
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      const result = await factory.createFromObject(rawDataContract);
      expect(result.toObject()).excluding('entropy').to.deep.equal(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      const alteredContract = dataContract.toObject();

      const resultSkipValidation = await factory.createFromObject(alteredContract, true);

      expect(resultSkipValidation.toObject()).excluding('entropy').to.deep.equal(alteredContract);
    });

    it('should throw an error if passed object is not valid', async () => {
      const alteredContract = dataContract.toObject();

      try {
        await factory.createFromObject(alteredContract);
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDataContractError);
        expect(e.getRawDataContract()).to.deep.equal(alteredContract);
        expect(e.getErrors()).to.have.length(1);

        const [consensusError] = e.getErrors();
        expect(consensusError).to.be.an.instanceOf(JsonSchemaError);
      }
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

    it('should throw InvalidDataContractError if the decoding fails with consensus error', async () => {
      try {
        // Casually break serialized data contract:
        serializedDataContract[0] = 3;
        await factory.createFromBuffer(serializedDataContract);
        expect.fail('should throw InvalidDataContractError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDataContractError);
        expect(e.getErrors()[0]).to.be.an.instanceOf(UnsupportedProtocolVersionError);
      }
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should return new DataContractCreateTransition with passed DataContract', async () => {
      // Create wasm version of DataContract
      const newDataContract = new DataContract(rawDataContract);
      newDataContract.setEntropy(dataContract.getEntropy());

      const result = await factory.createDataContractCreateTransition(newDataContract);

      expect(result.getProtocolVersion()).to.equal(getLatestProtocolVersion());
      expect(result.getEntropy()).to.deep.equal(dataContract.getEntropy());
      expect(result.getDataContract().toObject()).to.deep.equal(dataContract.toObject());
    });
  });
});
