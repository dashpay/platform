const { expect } = require('chai');
const getAccount = require('../../../../src/types/Wallet/methods/getAccount');
const { WALLET_TYPES } = require('../../../../src/CONSTANTS');

const exceptedException1 = 'getAccount expected index integer to be a property of accountOptions';


describe('Wallet - getAccount', () => {
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
    expect(() => getAccount.call(mockOpts, 0)).to.throw(exceptedException1);
    expect(timesCreateAccountCalled).to.equal(0);
    expect(timesAttachEventsCalled).to.equal(0);
  });
  it('should create an account when not existing and get it back', () => {
    let timesCreateAccountCalled = 0;
    let timesAttachEventsCalled = 0;
    const mockOpts1 = {
      accounts: [],
      storage: {},
      walletType: WALLET_TYPES.HDWALLET,
      createAccount: (opts = { index: 0 }) => {
        timesCreateAccountCalled += 1;
        const acc = {
          index: opts.index,
          storage: {
            attachEvents: () => timesAttachEventsCalled += 1,
          },
        };
        // This is actually done by Account class
        mockOpts1.accounts.push(acc);
        return acc;
      },
    };

    const acc = getAccount.call(mockOpts1);
    expect(acc.index).to.equal(0);
    expect(timesCreateAccountCalled).to.equal(1);
    expect(timesAttachEventsCalled).to.equal(1);
    const acc2 = getAccount.call(mockOpts1, { index: 0 });
    expect(acc2.index).to.equal(0);
    expect(timesCreateAccountCalled).to.equal(1);
    expect(timesAttachEventsCalled).to.equal(2);
    expect(acc2).to.deep.equal(acc);

    const acc3 = getAccount.call(mockOpts1, { index: 1 });
    expect(acc3.index).to.equal(1);
    expect(timesCreateAccountCalled).to.equal(2);
    expect(timesAttachEventsCalled).to.equal(3);
  });
});
