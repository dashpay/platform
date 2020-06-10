const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

describe('rpcServer', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let address;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();
    const coreAPI = dashCore.getApi();

    ({ result: address } = await coreAPI.getNewAddress());

    await coreAPI.generateToAddress(500, address);
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should return address summary', async () => {
    const result = await dapiClient.core.getAddressSummary(address);

    expect(result).to.be.an('object');
    expect(result.addrStr).to.equal(address);
  });

  it('should throw an error on invalid params', async () => {
    address = 'Xh7nD4vTUYAxy8GV7t1k8Er9ZKmxRBDcL';

    try {
      await dapiClient.core.getAddressSummary(address);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.name).to.equal('JsonRpcError');
      expect(e.message).contains('Invalid address');
    }
  });
});
