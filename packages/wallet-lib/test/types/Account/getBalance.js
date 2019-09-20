const { expect } = require('chai');
const mockedStore = require('../../fixtures/sirentonight-fullstore-snapshot-1562711703');
const getTotalBalance = require('../../../src/types/Account/methods/getTotalBalance');
const getConfirmedBalance = require('../../../src/types/Account/methods/getConfirmedBalance');
const getUnconfirmedBalance = require('../../../src/types/Account/methods/getUnconfirmedBalance');
const calculateDuffBalance = require('../../../src/types/Storage/methods/calculateDuffBalance');

let mockedWallet;
describe('Account - getTotalBalance', () => {
  before(() => {
    const storageHDW = {
      store: mockedStore,
      calculateDuffBalance,
      getStore: () => mockedStore,
      mappedAddress: {},
    };
    const walletId = Object.keys(mockedStore.wallets)[0];
    mockedWallet = Object.assign({
      walletId,
      accountIndex: 0,
      storage: storageHDW,
    });
  });
  it('should correctly get the balance', async () => {
    const balance = await getTotalBalance.call(mockedWallet);
    expect(balance).to.equal(184499999506);
  });
  it('should correctly get the balance confirmed only', async () => {
    const balance = await getConfirmedBalance.call(mockedWallet);
    expect(balance).to.equal(184499999506);
  });
  it('should correctly get the balance dash value instead of duff', async () => {
    const balanceTotalDash = await getTotalBalance.call(mockedWallet, false);
    const balanceUnconfDash = await getUnconfirmedBalance.call(mockedWallet, false);
    const balanceConfDash = await getConfirmedBalance.call(mockedWallet, false);

    expect(balanceTotalDash).to.equal(1844.99999506);
    expect(balanceUnconfDash).to.equal(0);
    expect(balanceConfDash).to.equal(1844.99999506);
  });
});
