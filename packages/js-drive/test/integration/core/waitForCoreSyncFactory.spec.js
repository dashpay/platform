const { startMongoDb, startDashCore } = require('@dashevo/dp-services-ctl');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('waitForCoreSyncFactory', function main() {
  this.timeout(90000);

  let mongoDB;
  let firstDashCore;
  let secondDashCore;
  let thirdDashCore;
  let container;
  let waitForCoreSync;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
    if (firstDashCore) {
      await firstDashCore.remove();
    }

    if (secondDashCore) {
      await secondDashCore.remove();
    }

    if (thirdDashCore) {
      await thirdDashCore.remove();
    }
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should wait until Dash Core in regtest mode with peers is synced', async () => {
    firstDashCore = await startDashCore();
    const { result: randomAddress } = await firstDashCore.getApi().getNewAddress();
    await firstDashCore.getApi().generateToAddress(1000, randomAddress);

    secondDashCore = await startDashCore();
    await secondDashCore.connect(firstDashCore);

    container = await createTestDIContainer(mongoDB, secondDashCore);
    waitForCoreSync = container.resolve('waitForCoreSync');

    await waitForCoreSync(() => {});

    const secondApi = secondDashCore.getApi();

    const {
      result: {
        blocks: currentBlockHeight,
        headers: currentHeadersNumber,
      },
    } = await secondApi.getBlockchainInfo();

    expect(currentBlockHeight).to.equal(1000);
    expect(currentHeadersNumber).to.equal(1000);
  });
});
