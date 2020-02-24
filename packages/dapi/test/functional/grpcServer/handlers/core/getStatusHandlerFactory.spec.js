const startDapi = require('@dashevo/dp-services-ctl/lib/services/startDapi');

describe('getStatusHandlerFactory', function main() {
  this.timeout(160000);

  let removeDapi;
  let dapiClient;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();

    await (dashCore.getApi()).generate(1000);
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should return status', async () => {
    const result = await dapiClient.getStatus();

    expect(result).to.have.a.property('coreVersion');
    expect(result).to.have.a.property('protocolVersion');
    expect(result).to.have.a.property('blocks');
    expect(result).to.have.a.property('timeOffset');
    expect(result).to.have.a.property('connections');
    expect(result).to.have.a.property('proxy');
    expect(result).to.have.a.property('difficulty');
    expect(result).to.have.a.property('testnet');
    expect(result).to.have.a.property('relayFee');
    expect(result).to.have.a.property('errors');
    expect(result).to.have.a.property('network');

    expect(result.blocks).to.equal(1000);
  });
});
