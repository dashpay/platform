const { startMongoDb, startDashCore } = require('@dashevo/dp-services-ctl');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('checkCoreSyncFinishedFactory', function main() {
  this.timeout(90000);

  let mongoDB;
  let firstDashCore;
  let secondDashCore;
  let container;
  let checkCoreSyncFinished;

  before(async () => {
    mongoDB = await startMongoDb();
    firstDashCore = await startDashCore();
    const { result: randomAddress } = await firstDashCore.getApi().getNewAddress();
    await firstDashCore.getApi().generateToAddress(1000, randomAddress);
  });

  after(async () => {
    await mongoDB.remove();
    await firstDashCore.remove();
    await secondDashCore.remove();
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should wait until Dash Core is synced', async () => {
    secondDashCore = await startDashCore();
    await secondDashCore.connect(firstDashCore);

    container = await createTestDIContainer(mongoDB, secondDashCore);
    checkCoreSyncFinished = container.resolve('checkCoreSyncFinished');

    await checkCoreSyncFinished(() => {});

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
