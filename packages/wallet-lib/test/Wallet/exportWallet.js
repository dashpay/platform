const { expect } = require('chai');
const exportWallet = require('../../src/Wallet/exportWallet');
const { WALLET_TYPES } = require('../../src/CONSTANTS');
const cR4t6ePrivateKey = require('../fixtures/cR4t6e_pk');
const knifeMnemonic = require('../fixtures/knifeeasily');

describe('Wallet - export Wallet', () => {
  it('should indicate on missing data', () => {
    const mockOpts1 = { };
    const mockOpts2 = { walletType: WALLET_TYPES.SINGLE_ADDRESS };
    const mockOpts3 = { walletType: WALLET_TYPES.HDWALLET };

    const exceptedException1 = 'Trying to export from an unknown wallet type';
    const exceptedException2 = 'No privateKey to export';
    const exceptedException3 = 'Wallet was not initiated with a mnemonic, can\'t export it';
    expect(() => exportWallet.call(mockOpts1)).to.throw(exceptedException1);
    expect(() => exportWallet.call(mockOpts2)).to.throw(exceptedException2);
    expect(() => exportWallet.call(mockOpts3)).to.throw(exceptedException3);
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
    expect(exportWallet.call(mockOpts1)).to.equal(cR4t6ePrivateKey.privateKey);
    expect(exportWallet.call(mockOpts2)).to.equal(knifeMnemonic.mnemonic);
  });
});
