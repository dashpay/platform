// const DashPlatformProtocol = require('@dashevo/dpp/lib/DashPlatformProtocol');
//
// const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const DataContractCreateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const DataContractFactory = require('@dashevo/dpp/lib/dataContract/DataContractFactory');

let { default: loadWasmDpp, DashPlatformProtocol, DataContract } = require('../../..');

describe('DataContractFacade', () => {
  let dpp;
  let dataContract;
  let dataContractFactory;

  before(async () => {
    ({ DashPlatformProtocol, DataContract } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dpp = new DashPlatformProtocol();

    dataContract = getDataContractFixture();

    dataContractFactory = new DataContractFactory(
      dpp,
      undefined,
      undefined,
    );
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContract.getOwnerId(),
        dataContract.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getOwnerId().toBuffer()).to.deep.equal(dataContract.getOwnerId());
      expect(result.getDocuments()).to.deep.equal(dataContract.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create DataContract from plain object', async () => {
      const result = await dpp.dataContract.createFromObject(dataContract.toObject());

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should create DataContract from string', async () => {
      const result = await dpp.dataContract.createFromBuffer(dataContract.toBuffer());

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should create DataContractCreateTransition from DataContract', () => {
      const stateTransition = dataContractFactory.createDataContractCreateTransition(dataContract);

      const result = dpp.dataContract.createDataContractCreateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });

  describe('validate', () => {
    it('should validate DataContract', async () => {
      const result = await dpp.dataContract.validate(dataContract.toObject());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
