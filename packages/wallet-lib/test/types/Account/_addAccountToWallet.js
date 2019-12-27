const { expect } = require('chai');
const addAccountToWallet = require('../../../src/types/Account/_addAccountToWallet');

describe('Account - addAccountToWallet', () => {
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
