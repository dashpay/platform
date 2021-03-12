const { startMongoDb, startDashCore } = require('@dashevo/dp-services-ctl');
const SimplifiedMNListStore = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListStore');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('updateSimplifiedMasternodeListFactory', function main() {
  this.timeout(190000);

  let mongoDB;
  let container;
  let dashCore;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
    if (dashCore) {
      await dashCore.remove();
    }
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should wait until SML will be retrieved', async () => {
    dashCore = await startDashCore();

    container = createTestDIContainer(mongoDB, dashCore);

    const simplifiedMasternodeList = container.resolve('simplifiedMasternodeList');

    expect(simplifiedMasternodeList.getStore()).to.equal(undefined);

    const { result: randomAddress } = await dashCore.getApi().getNewAddress();

    await dashCore.getApi().generateToAddress(1000, randomAddress);

    const updateSimplifiedMasternodeList = container.resolve('updateSimplifiedMasternodeList');

    await updateSimplifiedMasternodeList(1000);

    expect(simplifiedMasternodeList.getStore())
      .to.be.an.instanceOf(SimplifiedMNListStore);
  });
});
