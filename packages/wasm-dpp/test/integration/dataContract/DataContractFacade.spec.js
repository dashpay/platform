const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

const { default: loadWasmDpp } = require('../../..');
let {
  DashPlatformProtocol, DataContract, ValidationResult, DataContractValidator,
  DataContractFactory, DataContractCreateTransition,
} = require('../../..');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

describe('DataContractFacade', () => {
  let dpp;
  let dataContract;
  let dataContractFactory;
  let blsAdapter;
  let rawDataContract;

  before(async () => {
    ({
      DashPlatformProtocol, DataContract, ValidationResult,
      DataContractValidator, DataContractFactory, DataContractCreateTransition,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    blsAdapter = await getBlsAdapterMock();
    dpp = new DashPlatformProtocol(blsAdapter);

    dataContract = await getDataContractFixture();
    rawDataContract = dataContract.toObject();
    // rawDataContract.$id = Buffer.from(dataContract.getId());
    // rawDataContract.ownerId = Buffer.from(dataContract.getOwnerId());
    // rawDataContract.$ownerId = Buffer.from(dataContract.getOwnerId());

    const dataContractValidator = new DataContractValidator();
    dataContractFactory = new DataContractFactory(
      1,
      dataContractValidator,
    );

    // dataContractFactory = new DataContractFactory(
    //   { getProtocolVersion: () => 1 },
    //   undefined,
    //   undefined,
    // );
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContract.getOwnerId(),
        dataContract.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getOwnerId().toBuffer()).to.deep.equal(dataContract.getOwnerId().toBuffer());
      expect(result.getDocuments()).to.deep.equal(dataContract.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create DataContract from plain object', async() => {
      try {
        console.dir(rawDataContract);
        const result = await dpp.dataContract.createFromObject(rawDataContract);

        expect(result).to.be.an.instanceOf(DataContract);

        expect(result.toObject()).to.deep.equal(dataContract.toObject());
      } catch (e) {
        console.dir(e.getMessage());
        expect.fail();
      }
    });
  });

  describe('createFromBuffer', () => {
    it('should create DataContract from string', async() => {
      try {
        const contract = dpp.dataContract.create(
            dataContract.getOwnerId(),
            dataContract.getDocuments(),
        );

        const result = await dpp.dataContract.createFromBuffer(contract.toBuffer());

        expect(result).to.be.an.instanceOf(DataContract);

        expect(result.toObject()).to.deep.equal(dataContract.toObject());
      } catch (e) {
        console.log(e);
        expect.fail();
      }
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should create DataContractCreateTransition from DataContract', async () => {
      console.dir(dataContract.toObject());

      console.log('***********************');

      const stateTransition = await dataContractFactory.createDataContractCreateTransition(dataContract);

      const result = dpp.dataContract.createDataContractCreateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      console.dir(result.toObject());

      console.log('=====================');

      console.dir(stateTransition.toObject());

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });

  describe('validate', () => {
    it('should validate DataContract', async () => {
      const result = await dpp.dataContract.validate(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
