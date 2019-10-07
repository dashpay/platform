const { expect } = require('chai');

const { Account, Wallet } = require('../../../src');

let mnemonic;
let expectedIdentityPrivateKey;
let expectedIdentityPrivateKey1;
let walletMock;
let account;

describe('Account - getIdentityPrivateKey', () => {
  beforeEach(() => {
    mnemonic = 'during develop before curtain hazard rare job language become verb message travel';
    expectedIdentityPrivateKey = 'c2c687fb69dbabbc984f020021c40c1e8c96d40c1ce7de335ac55f84ef816098';
    expectedIdentityPrivateKey1 = '66680bff9a62e2bf1149536d4df62bb2c0c4b0ec99a556f9fe055cf94b94ea7b';
    walletMock = new Wallet({
      offlineMode: true,
      mnemonic,
    });
    account = new Account(walletMock);
  });

  afterEach(() => {
    walletMock.disconnect();
  });

  it('Should derive a key for identity for a given index', () => {
    const actualIdentityPrivateKey = account.getIdentityPrivateKey(0);
    const actualIdentityPrivateKey1 = account.getIdentityPrivateKey(1);

    expect(actualIdentityPrivateKey.toString()).to.be.equal(expectedIdentityPrivateKey);
    expect(actualIdentityPrivateKey1.toString()).to.be.equal(expectedIdentityPrivateKey1);
  });
});
