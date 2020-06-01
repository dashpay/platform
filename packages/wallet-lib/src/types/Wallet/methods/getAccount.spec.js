const { expect } = require('chai');
const getAccount = require('./getAccount');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const expectThrowsAsync = require('../../../utils/expectThrowsAsync');

const exceptedException1 = 'getAccount expected index integer to be a property of accountOptions';


describe('Wallet - getAccount', function suite() {
  this.timeout(10000);
  it('should warn on trying to pass arg as number', () => {
    let timesCreateAccountCalled = 0;
    let timesAttachEventsCalled = 0;
    const mockOpts = {
      accounts: [],
      storage: {},
      walletType: WALLET_TYPES.HDWALLET,
      createAccount: (opts = { index: 0 }) => {
        timesCreateAccountCalled += 1;
        return {
          index: opts.index,
          storage: {
            attachEvents: () => timesAttachEventsCalled += 1,
          },
        };
      },
    };
    expectThrowsAsync(async () => await getAccount.call(mockOpts, 0),exceptedException1);
    expect(timesCreateAccountCalled).to.equal(0);
    expect(timesAttachEventsCalled).to.equal(0);
  });
  it('should create an account when not existing and get it back', async () => {
    let timesCreateAccountCalled = 0;
    const mockOpts1 = {
      accounts: [],
      storage: {},
      walletType: WALLET_TYPES.HDWALLET,
      createAccount: (opts = { index: 0 }) => {
        timesCreateAccountCalled += 1;
        const acc = {
          index: opts.index,
        };
        // This is actually done by Account class
        mockOpts1.accounts.push(acc);
        return acc;
      },
    };

    const acc = await getAccount.call(mockOpts1);
    expect(acc.index).to.equal(0);
    expect(timesCreateAccountCalled).to.equal(1);
    const acc2 = await getAccount.call(mockOpts1, { index: 0 });
    expect(acc2.index).to.equal(0);
    expect(timesCreateAccountCalled).to.equal(1);
    expect(acc2).to.deep.equal(acc);

    const acc3 = await getAccount.call(mockOpts1, { index: 1 });
    expect(acc3.index).to.equal(1);
    expect(timesCreateAccountCalled).to.equal(2);
  });
});
