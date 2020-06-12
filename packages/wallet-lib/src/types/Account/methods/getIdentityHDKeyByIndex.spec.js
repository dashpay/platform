const { expect } = require('chai');

const { Account, Wallet } = require('../../../index');

let mnemonic;
let expectedIdentityHDKey0_0;
let expectedIdentityHDKey0_1;
let expectedIdentityHDKey1_0;
let expectedIdentityPrivateKey0_0;
let expectedIdentityPrivateKey0_1;
let expectedIdentityPrivateKey1_0;
let walletMock;
let account;

describe('Account - getIdentityHDKeyByIndex', function suite() {
  this.timeout(10000);
  beforeEach(() => {
    mnemonic = 'during develop before curtain hazard rare job language become verb message travel';


    expectedIdentityHDKey0_0 = 'tprv8oH6ANFZK6Ap3rp4bzcafECooaK2Hj7J7bvqBug5UYHYGXaQVK9CfVeVgH4Y4HmLghrPU64bRbzQk82cCv6Sm4E4JjsTRb8WJ75FG2Qwg43';
    expectedIdentityHDKey0_1 = 'tprv8oH6ANFZK6Ap7Qik5WRLovyK32LPCaqsZGeeh7AMFwxgBTABrDfigb399HTA1KT7vjYVurMvJo4REpcGorKPr2LC6SsSBwVdogg5UYW6C6n';
    expectedIdentityHDKey1_0 = 'tprv8oxtsevLHCPBCyTVpzhiyYnABQxhJwGp721eAT9dxb8VPBch6kp265Ry4qdL4mcktzLwPF3sZnmhTMd2oqkmSXWK6NHbwEMPFgKv6wUCGBW';

    expectedIdentityPrivateKey0_0 = '483b7555c139931da369c53d0cbe55bd5fe4461a713504fc777963f9062075e4';
    expectedIdentityPrivateKey0_1 = 'f2c7ea82ffa0007ab6f27f53cfce5137d597fffa8a6e38cadefb6f9953e88e30';
    expectedIdentityPrivateKey1_0 = '80ae8ea14f36be65fd53309115ff26e4968686b884abad902b8d5f66637a235a';
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
    const actualIdentityHDKey0_0 = account.getIdentityHDKeyByIndex(0, 0);
    const actualIdentityHDKey0_1 = account.getIdentityHDKeyByIndex(0, 1);
    const actualIdentityHDKey1_0 = account.getIdentityHDKeyByIndex(1, 0);

    expect(actualIdentityHDKey0_0.toString()).to.be.equal(expectedIdentityHDKey0_0);
    expect(actualIdentityHDKey0_1.toString()).to.be.equal(expectedIdentityHDKey0_1);
    expect(actualIdentityHDKey1_0.toString()).to.be.equal(expectedIdentityHDKey1_0);

    expect(actualIdentityHDKey0_0.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey0_0);
    expect(actualIdentityHDKey0_1.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey0_1);
    expect(actualIdentityHDKey1_0.privateKey.toString()).to.be.equal(expectedIdentityPrivateKey1_0);
  });
});
