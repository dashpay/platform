const { expect } = require('chai');
const generateNewWalletId = require('../../../src/types/Wallet/methods/generateNewWalletId');
const knifeMnemonic = require('../../fixtures/knifeeasily');
const gatherSail = require('../../fixtures/gathersail');
const cR4t6ePrivateKey = require('../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../src/CONSTANTS');

describe('Wallet - generateNewWalletId', () => {
  it('should indicate on missing data', () => {
    const mockOpts1 = { };
    const mockOpts2 = { walletType: WALLET_TYPES.HDWALLET };
    const mockOpts3 = { walletType: WALLET_TYPES.SINGLE_ADDRESS };

    const exceptedException1 = 'Cannot generate a walletId : No HDPrivateKey found';
    const exceptedException3 = 'Cannot generate a walletId : No privateKey found';
    expect(() => generateNewWalletId.call(mockOpts1)).to.throw(exceptedException1);
    expect(() => generateNewWalletId.call(mockOpts2)).to.throw(exceptedException1);
    expect(() => generateNewWalletId.call(mockOpts3)).to.throw(exceptedException3);
  });
  it('should generate a wallet id from HDWallet', () => {
    const mockOptsMainnet = { HDPrivateKey: knifeMnemonic.HDRootPrivateKeyMainnet };
    const mockOptsTestnet = { HDPrivateKey: knifeMnemonic.HDRootPrivateKeyTestnet };

    const walletId1 = generateNewWalletId.call(mockOptsMainnet);
    expect(walletId1).to.length(10);
    expect(walletId1).to.equal(knifeMnemonic.HDPrivateKeyMainnetWalletId);

    const walletId2 = generateNewWalletId.call(mockOptsTestnet);
    expect(walletId2).to.length(10);
    expect(walletId2).to.equal(knifeMnemonic.HDPrivateKeyTestnetWalletId);
  });
  it('should generate a wallet id from HDPubKey', () => {
    const mockOptsTestnet = {
      walletType: WALLET_TYPES.HDEXTPUBLIC,
      HDExtPublicKey: gatherSail.testnet.external.hdpubkey,
    };

    const walletId1 = generateNewWalletId.call(mockOptsTestnet);
    expect(walletId1).to.length(10);
    expect(walletId1).to.equal(gatherSail.testnet.external.walletId);
  });
  it('should generate a wallet id from single pk', () => {
    const mockOpts = {
      walletType: WALLET_TYPES.SINGLE_ADDRESS,
      privateKey: cR4t6ePrivateKey.privateKey,
    };

    const walletId = generateNewWalletId.call(mockOpts);
    expect(walletId).to.length(10);
    expect(walletId).to.equal(cR4t6ePrivateKey.walletIdTestnet);
  });
});
