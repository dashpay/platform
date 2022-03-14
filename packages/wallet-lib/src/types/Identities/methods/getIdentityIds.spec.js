const { expect } = require('chai');
const mockedStore = require('../../../../fixtures/sirentonight-fullstore-snapshot-1562711703');
const getIdentityIds = require('./getIdentityIds');
const WalletStore = require("../../WalletStore/WalletStore");

let mockedWallet;
let fetchTransactionInfoCalledNb = 0;
describe('Wallet#getIdentityIds', function suite() {
  this.timeout(10000);
  before(() => {
    const walletId = Object.keys(mockedStore.wallets)[0];
    const walletStore = new WalletStore(walletId)
    const identityIds = mockedStore.wallets[walletId].identityIds;
    identityIds.forEach((id, i) => {
      walletStore.insertIdentityIdAtIndex(id, i)
    })

    const storageHDW = {
      getWalletStore: () => walletStore
    };

    mockedWallet = {
      walletId,
      index: 0,
      storage: storageHDW,
      transport: {
        getTransaction: () => fetchTransactionInfoCalledNb += 1,
      },
    };
  });
  it('should filter empty indexes', async () => {
    const identityIds = await getIdentityIds.call(mockedWallet);
    expect(identityIds).to.deep.equal([
      "9Gk9T5mJY9j3dDX1D1tG5WYaV8g6zQTS2ocFFXe6NCrq",
      "HZJywfYZ87fdJFLkp7wtnTfS29zpvR63f21gqaajLYx6"
    ]);
  });
});
