const { expect } = require('chai');
const addAccountToWallet = require('./_addAccountToWallet');

describe('Account - addAccountToWallet', function suite() {
  this.timeout(10000);
  it('should add an account to a wallet', () => {
    const wallet = {
      accounts: [],
    };
    const mockAcc = {
      label: 'mockedAccount',
      index: 0,
    };
    addAccountToWallet(mockAcc, wallet);
    expect(wallet.accounts).to.deep.equal([{ label: 'mockedAccount', index: 0 }]);
  });
});
