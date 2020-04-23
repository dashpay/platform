const { expect } = require('chai');

const { Account, Wallet } = require('../../../index');

let mnemonic;
let expectedIdentityHDKey;
let expectedIdentityPrivateKey;
let expectedIdentityHDKey1;
let expectedIdentityPrivateKey1;
let walletMock;
let account;

describe('Account - getIdentityHDKey', () => {
  beforeEach(() => {
    mnemonic = 'during develop before curtain hazard rare job language become verb message travel';


    expectedIdentityHDKey = 'tprv8neTemzR4tDRNCgSvzAnZuzpqC1qiBYQJpsdAZijdMVzhb9zLfPp7A9sY8FeES32rmfAPtiJMWiwTANH2khMvnbH5SYCmPAyr9n6nkRdD8u';
    expectedIdentityHDKey1 = 'tprv8neTemzR4tDRPzdaAFnwH1StohsurPqsDBMsC2xmRE4t8hE9EAbFPrGpssCXPQCSs39ndQJCY7FaTwvB8mhv9f6otnXBoJYa3MQNZeynaFv';

    expectedIdentityPrivateKey = '34c2a49feb85f59eec9fd953d5ff1815af70bc7173ebbd26bc5af810b23961c7';
    expectedIdentityPrivateKey1 = '0611f36b51b8bf145edf26942b869a52195d16a773ea1729e64f2c76b2324578';
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
    const actualIdentityHDKey = account.getIdentityHDKey(0);
    const actualIdentityHDKey1 = account.getIdentityHDKey(1);

    expect(actualIdentityHDKey.toString()).to.be.equal(expectedIdentityHDKey);
    expect(actualIdentityHDKey1.toString()).to.be.equal(expectedIdentityHDKey1);

    expect(actualIdentityHDKey.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey);
    expect(actualIdentityHDKey1.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey1);
  });
});
