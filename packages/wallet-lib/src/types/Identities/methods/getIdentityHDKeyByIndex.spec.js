const { expect } = require('chai');

const { Wallet, Identities } = require('../../../index');

let mnemonic;
let expectedIdentityHDKey0_0;
let expectedIdentityHDKey0_1;
let expectedIdentityHDKey1_0;
let expectedIdentityPrivateKey0_0;
let expectedIdentityPrivateKey0_1;
let expectedIdentityPrivateKey1_0;
let wallet;
let identities;
describe('Identities#getIdentityHDKeyByIndex', function suite() {
  this.timeout(10000);
  beforeEach(() => {
    mnemonic = 'during develop before curtain hazard rare job language become verb message travel';


    expectedIdentityHDKey0_0 = 'tprv8nwXBDgtqkF6xZjxESRmMcmyo8LeJ7YnhEZNYrGBUVDtDbxdtjiQQ5pyVigvrep81EJWenD3BEdCV5Yrhah2tbnzjM5Dq9bnmDvX7yyRHRr';
    expectedIdentityHDKey0_1 = 'tprv8nwXBDgtqkF6z6Da9eSrw29t3qVcHqWTLzw5oFVzXnuxwhRF5RtMmc3LqGMD6NmShVUd4dkbs86PB4pZVQ7xWgg2BLK4Kqm7TDTct4YDifH';
    expectedIdentityHDKey1_0 = 'tprv8oNTEowGNFSSD6Ne3aR9hQXFT2hmvf4F9kgjbbrKmCyeBWbuH9an16tPtKrHtkbAyHofhfGa1Go6a4bZQukJ8qS657PJQEMg3Sq3Z22UnH6';

    expectedIdentityPrivateKey0_0 = '6fcf62a14d7c452a77dee426a534b7c92cbb13a41c3b7f75700519e339ef09dc';
    expectedIdentityPrivateKey0_1 = '5e07be03de51b0c5f7af8d60074819e2cf4bdce8eb47e59c18295e151528390f';
    expectedIdentityPrivateKey1_0 = '276d1d2aa6df3c3b7d9da967641769eddd7e81055833b90a79cdb1b433dd18e5';
    wallet = new Wallet({
      offlineMode: true,
      mnemonic,
    });
    identities = new Identities(wallet);
  });

  afterEach(() => {
    wallet.disconnect();
  });

  it('Should derive a key for identity for a given index', () => {
    const actualIdentityHDKey0_0 = identities.getIdentityHDKeyByIndex(0, 0);
    const actualIdentityHDKey0_1 = identities.getIdentityHDKeyByIndex(0, 1);
    const actualIdentityHDKey1_0 = identities.getIdentityHDKeyByIndex(1, 0);

    expect(actualIdentityHDKey0_0.toString()).to.be.equal(expectedIdentityHDKey0_0);
    expect(actualIdentityHDKey0_1.toString()).to.be.equal(expectedIdentityHDKey0_1);
    expect(actualIdentityHDKey1_0.toString()).to.be.equal(expectedIdentityHDKey1_0);

    expect(actualIdentityHDKey0_0.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey0_0);
    expect(actualIdentityHDKey0_1.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey0_1);
    expect(actualIdentityHDKey1_0.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey1_0);
  });
});
