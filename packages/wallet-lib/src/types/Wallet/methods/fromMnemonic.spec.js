const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const fromMnemonic = require('./fromMnemonic');
const knifeFixture = require('../../../../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../../CONSTANTS');

describe('Wallet - fromMnemonic', function suite() {
  this.timeout(10000);
  it('should indicate missing data', () => {
    const mockOpts1 = {};
    const exceptedException1 = 'Expected a valid mnemonic (typeof String or Mnemonic)';
    expect(() => fromMnemonic.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from mnemonic', () => {
    const self1 = {
      network: 'livenet',
    };
    fromMnemonic.call(self1, knifeFixture.mnemonic, 'livenet');
    expect(self1.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(knifeFixture.mnemonic);
    expect(self1.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(new Dashcore.HDPrivateKey(self1.HDPrivateKey)).to.equal(self1.HDPrivateKey);

    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain.network).to.equal('livenet');
    expect(keyChain.rootKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);


    const self2 = {};
    fromMnemonic.call(self2, knifeFixture.mnemonic);
    expect(self2.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(knifeFixture.mnemonic);

    const keyChain2 = self2.keyChainStore.getMasterKeyChain()
    expect(keyChain2.network).to.equal('testnet');
    expect(self2.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
    expect(new Dashcore.HDPrivateKey(self2.HDPrivateKey)).to.equal(self2.HDPrivateKey);
    expect(keyChain2.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain2.rootKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { mnemonic: 'knife easily prosper input concert merge prepare autumn pen blood glance chair' },
      { mnemonic: false },
      { mnemonic: true },
      { mnemonic: 0 },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromMnemonic.call(self, invalidInput)).to.throw('Expected a valid mnemonic (typeof String or Mnemonic)');
    });
  });
});
describe('Wallet - fromMnemonic - with passphrase', function suite() {
  this.timeout(10000);
  it('should correctly works with passphrase', () => {
    const self1 = {
    };
    fromMnemonic.call(self1, knifeFixture.mnemonic, 'livenet', knifeFixture.passphrase);
    expect(self1.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(knifeFixture.mnemonic);
    expect(self1.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootEncryptedPrivateKeyMainnet);
    expect(new Dashcore.HDPrivateKey(self1.HDPrivateKey)).to.equal(self1.HDPrivateKey);
    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain.network).to.equal('livenet');
    expect(keyChain.rootKey.toString()).to.equal(knifeFixture.HDRootEncryptedPrivateKeyMainnet);

    const path1 = 'm/44\'/5\'/0\'/0/0';
    const pubKey1 = keyChain.getForPath(path1).key.publicKey.toAddress();
    expect(new Dashcore.Address(pubKey1).toString()).to.equal('Xq3zjky18WjwAHpLgGLasvX5g8TeLRKaxt');

    const self2 = {
    };
    fromMnemonic.call(self2, knifeFixture.mnemonic, 'testnet', knifeFixture.passphrase);
    expect(self2.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(knifeFixture.mnemonic);
    expect(self2.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootEncryptedPrivateKeyTestnet);
    expect(new Dashcore.HDPrivateKey(self2.HDPrivateKey)).to.equal(self2.HDPrivateKey);
    const keyChain2 = self2.keyChainStore.getMasterKeyChain()
    expect(keyChain2.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain2.network).to.equal('testnet');
    expect(keyChain2.rootKey.toString()).to.equal(knifeFixture.HDRootEncryptedPrivateKeyTestnet);

    const path2 = 'm/44\'/1\'/0\'/0/0';
    const pubKey2 = keyChain2.getForPath(path2).key.publicKey.toAddress();
    expect(new Dashcore.Address(pubKey2, 'testnet').toString()).to.equal('yWYCH9XDRnpdNxh67jQJFkovToBVwWr8Ck');
  });
});
