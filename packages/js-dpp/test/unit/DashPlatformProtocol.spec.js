const DashPlatformProtocol = require('../../lib/DashPlatformProtocol');

const createDataProviderMock = require('../../lib/test/mocks/createDataProviderMock');

describe('DashPlatformProtocol', () => {
  let dpp;
  let dataProvider;

  beforeEach(function beforeEach() {
    dataProvider = createDataProviderMock(this.sinonSandbox);

    dpp = new DashPlatformProtocol({
      dataProvider,
    });
  });

  describe('getDataProvider', () => {
    it('should return Data Provider', () => {
      const result = dpp.getDataProvider();

      expect(result).to.equal(dataProvider);
    });
  });
});
