const { expect } = require('chai');
const Wallet = require('../../../../src/types/Wallet/Wallet');
const exportWallet = require('../../../../src/types/Wallet/methods/exportWallet');
const { WALLET_TYPES } = require('../../../../src/CONSTANTS');
const cR4t6ePrivateKey = require('../../../fixtures/cR4t6e_pk');
const knifeMnemonic = require('../../../fixtures/knifeeasily');

describe('Wallet - export Wallet', () => {
  it('should indicate on missing data', () => {
    const mockOpts1 = {};
    const mockOpts2 = { walletType: WALLET_TYPES.SINGLE_ADDRESS };
    const mockOpts3 = { walletType: WALLET_TYPES.HDWALLET };

    const exceptedException1 = 'Trying to export from an unknown wallet type';
    const exceptedException2 = 'No PrivateKey to export';
    const exceptedException3 = 'Wallet was not initiated with a mnemonic, can\'t export it.';
    expect(() => exportWallet.call(mockOpts1)).to.throw(exceptedException1);
    expect(() => exportWallet.call(mockOpts2)).to.throw(exceptedException2);
    expect(() => exportWallet.call(mockOpts3)).to.throw(exceptedException2);
    expect(() => exportWallet.call(mockOpts3, 'mnemonic')).to.throw(exceptedException3);
  });
  it('should export a privateKey', () => {
    const mockOpts1 = {
      walletType: WALLET_TYPES.SINGLE_ADDRESS,
      privateKey: cR4t6ePrivateKey.privateKey,
    };
    const mockOpts2 = {
      walletType: WALLET_TYPES.HDWALLET,
      mnemonic: knifeMnemonic.mnemonic,
    };
    const mockOpts3 = {
      walletType: WALLET_TYPES.HDWALLET,
      HDPrivateKey: knifeMnemonic.HDRootPrivateKeyMainnet,
    };
    expect(exportWallet.call(mockOpts1)).to.equal(cR4t6ePrivateKey.privateKey);
    expect(exportWallet.call(mockOpts2)).to.equal(knifeMnemonic.mnemonic);
    expect(exportWallet.call(mockOpts3)).to.equal(knifeMnemonic.HDRootPrivateKeyMainnet);
  });
});
describe('Wallet - exportWallet - integration', () => {
  describe('fromMnemonic', () => {
    const wallet = new Wallet({
      offlineMode: true,
      mnemonic: knifeMnemonic.mnemonic,
    });
    it('should works as expected', () => {
      const exceptedException = 'Tried to export to invalid output : seed';
      expect(wallet.exportWallet()).to.equal(knifeMnemonic.mnemonic);
      expect(wallet.exportWallet('mnemonic')).to.equal(knifeMnemonic.mnemonic);
      expect(wallet.exportWallet('HDPrivateKey')).to.equal(knifeMnemonic.HDRootPrivateKeyTestnet);
      expect(() => wallet.exportWallet('seed')).to.throw(exceptedException);
    });
    after(() => {
      wallet.disconnect();
    });
  });
  describe('fromSeed', () => {
    const wallet = new Wallet({
      offlineMode: true,
      seed: knifeMnemonic.seed,
    });
    it('should works as expected', () => {
      const exceptedException = "Wallet was not initiated with a mnemonic, can't export it.";
      const exceptedException2 = 'Tried to export to invalid output : seed';

      expect(wallet.exportWallet()).to.equal(knifeMnemonic.HDRootPrivateKeyTestnet);
      expect(() => wallet.exportWallet('mnemonic')).to.throw(exceptedException);
      expect(() => wallet.exportWallet('seed')).to.throw(exceptedException2);
      expect(wallet.exportWallet('HDPrivateKey')).to.equal(knifeMnemonic.HDRootPrivateKeyTestnet);
    });
    after(() => {
      wallet.disconnect();
    });
  });
  describe('fromHDPrivateKey', () => {
    const wallet = new Wallet({
      offlineMode: true,
      HDPrivateKey: knifeMnemonic.HDRootPrivateKeyTestnet,
    });
    it('should works as expected', () => {
      const exceptedException = "Wallet was not initiated with a mnemonic, can't export it.";
      const exceptedException2 = 'Tried to export to invalid output : seed';

      expect(wallet.exportWallet()).to.equal(knifeMnemonic.HDRootPrivateKeyTestnet);
      expect(() => wallet.exportWallet('mnemonic')).to.throw(exceptedException);
      expect(() => wallet.exportWallet('seed')).to.throw(exceptedException2);
      expect(wallet.exportWallet('HDPrivateKey')).to.equal(knifeMnemonic.HDRootPrivateKeyTestnet);
    });
    after(() => {
      wallet.disconnect();
    });
  });
  describe('fromHDPublicKey', () => {
    const wallet = new Wallet({
      offlineMode: true,
      HDPublicKey: knifeMnemonic.HDRootPublicKeyMainnet,
    });
    it('should works as expected', () => {
      const exceptedException = 'Tried to export to invalid output : mnemonic';
      const exceptedException2 = 'Tried to export to invalid output : seed';
      const exceptedException3 = 'Tried to export to invalid output : HDPrivateKey';

      expect(wallet.exportWallet()).to.equal(knifeMnemonic.HDRootPublicKeyMainnet);
      expect(() => wallet.exportWallet('mnemonic')).to.throw(exceptedException);
      expect(() => wallet.exportWallet('seed')).to.throw(exceptedException2);
      expect(() => wallet.exportWallet('HDPrivateKey')).to.throw(exceptedException3);
      expect(wallet.exportWallet('HDPublicKey')).to.equal(knifeMnemonic.HDRootPublicKeyMainnet);
    });
    after(() => {
      wallet.disconnect();
    });
  });
});
