const getDataContractJSFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

const { default: loadWasmDpp } = require('../../..');
let {
  DashPlatformProtocol, DataContract, ValidationResult, DataContractValidator,
  DataContractFactory, DataContractCreateTransition,
} = require('../../..');

describe('DataContractFacade', () => {
  let dpp;
  let dataContractJs;
  let dataContractFactory;
  let blsAdapter;
  let rawDataContract;
  let dataContractWasm;

  before(async () => {
    ({
      DashPlatformProtocol, DataContract, ValidationResult,
      DataContractValidator, DataContractFactory, DataContractCreateTransition,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    blsAdapter = await getBlsAdapterMock();
    dpp = new DashPlatformProtocol(blsAdapter);

    dataContractJs = await getDataContractJSFixture();
    rawDataContract = dataContractJs.toObject();

    const dataContractValidator = new DataContractValidator();
    dataContractFactory = new DataContractFactory(
      1,
      dataContractValidator,
    );

    dataContractWasm = await dataContractFactory.createFromObject(rawDataContract);
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContractJs.getOwnerId(),
        dataContractJs.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getOwnerId().toBuffer()).to.deep.equal(dataContractJs.getOwnerId().toBuffer());
      expect(result.getDocuments()).to.deep.equal(dataContractJs.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create DataContract from plain object', async () => {
      const result = await dpp.dataContract.createFromObject(rawDataContract);

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toObject()).to.deep.equal(dataContractJs.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should create DataContract from string', async () => {
      const contract = dpp.dataContract.create(
        dataContractJs.getOwnerId(),
        dataContractJs.getDocuments(),
      );

      const result = await dpp.dataContract.createFromBuffer(contract.toBuffer());

      expect(result).to.be.an.instanceOf(DataContract);
      expect(result.toObject()).to.deep.equal(contract.toObject());
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should create DataContractCreateTransition from DataContract', async () => {
      const stateTransition = await dataContractFactory
        .createDataContractCreateTransition(dataContractWasm);

      const result = dpp.dataContract.createDataContractCreateTransition(dataContractWasm);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });

  describe('validate', () => {
    it('should validate DataContract', async () => {
      const result = await dpp.dataContract.validate(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.getErrors().length).to.be.equal(0);
    });
  });
});
