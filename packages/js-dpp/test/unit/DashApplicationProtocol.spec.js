const DashApplicationProtocol = require('../../lib/DashApplicationProtocol');

const getDapContractFixture = require('../../lib/test/fixtures/getDapContractFixture');

const createDataProviderMock = require('../../lib/test/mocks/createDataProviderMock');

describe('DashApplicationProtocol', () => {
  let dap;
  let userId;
  let dapContract;
  let dataProvider;

  beforeEach(function beforeEach() {
    userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';
    dapContract = getDapContractFixture();
    dataProvider = createDataProviderMock(this.sinonSandbox);

    dap = new DashApplicationProtocol();
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      const result = dap.setUserId(userId);

      expect(result).to.be.instanceOf(DashApplicationProtocol);

      expect(dap.userId).to.be.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      dap.userId = userId;

      const result = dap.getUserId();

      expect(result).to.be.equal(userId);
    });
  });

  describe('setDapContract', () => {
    it('should set User ID', () => {
      const result = dap.setDapContract(dapContract);

      expect(result).to.be.instanceOf(DashApplicationProtocol);

      expect(dap.dapContract).to.be.equal(dapContract);
    });
  });

  describe('getDapContract', () => {
    it('should return DAP Contract', () => {
      dap.dapContract = dapContract;

      const result = dap.getDapContract();

      expect(result).to.be.equal(dapContract);
    });
  });

  describe('setDataProvider', () => {
    it('should set Data Provider', () => {
      const result = dap.setDataProvider(dataProvider);

      expect(result).to.be.instanceOf(DashApplicationProtocol);

      expect(dap.dataProvider).to.be.equal(dataProvider);
    });
  });

  describe('getDataProvider', () => {
    it('should return Data Provider', () => {
      dap.dataProvider = dataProvider;

      const result = dap.getDataProvider();

      expect(result).to.be.equal(dataProvider);
    });
  });
});
