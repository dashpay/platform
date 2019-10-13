const DashPlatformProtocol = require('../../lib/DashPlatformProtocol');

const getDataContractFixture = require('../../lib/test/fixtures/getDataContractFixture');

const createDataProviderMock = require('../../lib/test/mocks/createDataProviderMock');

describe('DashPlatformProtocol', () => {
  let dpp;
  let userId;
  let dataContract;
  let dataProvider;

  beforeEach(function beforeEach() {
    userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';
    dataContract = getDataContractFixture();
    dataProvider = createDataProviderMock(this.sinonSandbox);

    dpp = new DashPlatformProtocol();
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      const result = dpp.setUserId(userId);

      expect(result).to.be.an.instanceOf(DashPlatformProtocol);

      expect(dpp.getUserId()).to.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      dpp.setUserId(userId);

      const result = dpp.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('setDataContract', () => {
    it('should set User ID', () => {
      const result = dpp.setDataContract(dataContract);

      expect(result).to.be.an.instanceOf(DashPlatformProtocol);

      expect(dpp.getDataContract()).to.equal(dataContract);
    });
  });

  describe('getDataContract', () => {
    it('should return Data Contract', () => {
      dpp.setDataContract(dataContract);

      const result = dpp.getDataContract();

      expect(result).to.equal(dataContract);
    });
  });

  describe('setDataProvider', () => {
    it('should set Data Provider', () => {
      const result = dpp.setDataProvider(dataProvider);

      expect(result).to.be.an.instanceOf(DashPlatformProtocol);

      expect(dpp.getDataProvider()).to.equal(dataProvider);
    });
  });

  describe('getDataProvider', () => {
    it('should return Data Provider', () => {
      dpp.setDataProvider(dataProvider);

      const result = dpp.getDataProvider();

      expect(result).to.equal(dataProvider);
    });
  });
});
