const { startDashCore } = require('@dashevo/dp-services-ctl');
const SimplifiedMNListStore = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListStore');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('updateSimplifiedMasternodeListFactory', function main() {
  this.timeout(190000);

  let container;
  let dashCore;

  after(async () => {
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

    container = await createTestDIContainer(dashCore);

    const simplifiedMasternodeList = container.resolve('simplifiedMasternodeList');

    expect(simplifiedMasternodeList.getStore()).to.equal(undefined);

    const { result: randomAddress } = await dashCore.getApi().getNewAddress();

    await dashCore.getApi().generateToAddress(1000, randomAddress);

    const updateSimplifiedMasternodeList = container.resolve('updateSimplifiedMasternodeList');

    await updateSimplifiedMasternodeList(1000);

    expect(simplifiedMasternodeList.getStore())
      .to.be.an.instanceOf(SimplifiedMNListStore);
  });

  it('should synchronizeMasternodeIdentities by smlMaxListsLimit number of blocks', async () => {
    dashCore = await startDashCore();

    container = await createTestDIContainer(dashCore);

    const simplifiedMasternodeList = container.resolve('simplifiedMasternodeList');
    const updateSimplifiedMasternodeList = container.resolve('updateSimplifiedMasternodeList');
    const synchronizeMasternodeIdentities = container.resolve('synchronizeMasternodeIdentities');
    const smlMaxListsLimit = container.resolve('smlMaxListsLimit');

    const api = dashCore.getApi();
    const { result: randomAddress } = await api.getNewAddress();

    await api.generateToAddress(600, randomAddress);

    let blockNumber = 500;

    await updateSimplifiedMasternodeList(blockNumber);
    await synchronizeMasternodeIdentities(blockNumber);

    expect(simplifiedMasternodeList.getStore())
      .to.be.an.instanceOf(SimplifiedMNListStore);

    blockNumber += smlMaxListsLimit;

    await updateSimplifiedMasternodeList(blockNumber);
    await synchronizeMasternodeIdentities(blockNumber);

    expect(simplifiedMasternodeList.getStore())
      .to.be.an.instanceOf(SimplifiedMNListStore);

    blockNumber += smlMaxListsLimit;

    await updateSimplifiedMasternodeList(blockNumber);
    await synchronizeMasternodeIdentities(blockNumber);

    expect(simplifiedMasternodeList.getStore())
      .to.be.an.instanceOf(SimplifiedMNListStore);
  });
});
