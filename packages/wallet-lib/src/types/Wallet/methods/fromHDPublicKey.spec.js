const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const Wallet = require('../Wallet');
const fromHDPublicKey = require('./fromHDPublicKey');
const gatherSail = require('../../../../fixtures/gathersail');
const { WALLET_TYPES } = require('../../../CONSTANTS');
/**
 * Theses first set of data labeled gatherSail correspond to the following mnemonic:
 * gather sail face invite together focus waste barely excuse slide harbor hint
 *
 *
 * @type {string}
 */
describe('Wallet - HDPublicKey', () => {
  const gatherTestnet = gatherSail.testnet;
  it('should detect wrong parameters', () => {
    const mockOpts1 = {};
    const exceptedException1 = 'Expected a valid HDPublicKey (typeof HDPublicKey or String)';
    expect(() => fromHDPublicKey.call(mockOpts1)).to.throw(exceptedException1);
    expect(() => fromHDPublicKey.call(mockOpts1, gatherTestnet.external.hdprivkey)).to.throw(exceptedException1);
    expect(() => fromHDPublicKey.call(mockOpts1, gatherTestnet.mnemonic)).to.throw(exceptedException1);
    expect(() => fromHDPublicKey.call(mockOpts1, 'cR4t6evwVZoCp1JsLk4wURK4UmBCZzZotNzn9T1mhBT19SH9JtNt')).to.throw(exceptedException1);
  });
  it('should work from a valid HDPubKey', () => {
    const mockOpts1 = {};
    fromHDPublicKey.call(mockOpts1, gatherTestnet.external.hdpubkey);

    expect(mockOpts1.walletType).to.equal(WALLET_TYPES.HDPUBLIC);
    expect(mockOpts1.mnemonic).to.equal(null);
    expect(mockOpts1.HDPublicKey.toString()).to.equal(gatherTestnet.external.hdpubkey);
    expect(new Dashcore.HDPublicKey(mockOpts1.HDPublicKey)).to.equal(mockOpts1.HDPublicKey);
    expect(mockOpts1.keyChain.type).to.equal('HDPublicKey');
    expect(mockOpts1.keyChain.HDPublicKey).to.deep.equal(Dashcore.HDPublicKey(gatherTestnet.external.hdpubkey));
    expect(mockOpts1.keyChain.keys).to.deep.equal({});
  });
  it('should work from a HDPubKey', () => {
    const wallet1 = new Wallet(
      { HDPublicKey: gatherTestnet.external.hdpubkey, offlineMode: true },
    );

    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.HDPUBLIC);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('HDPublicKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(gatherTestnet.external.walletId);

    expect(wallet1.exportWallet()).to.be.equal(gatherTestnet.external.hdpubkey);

    const account = wallet1.getAccount();
    const unusedAddress = account.getUnusedAddress();
    const expectedUnused = {
      path: "m/44'/1'/0'/0/0",
      index: 0,
      address: 'yNJ3xxTXXBBf39VfMBbBuLH2k57uAwxBxj',
      transactions: [],
      balanceSat: 0,
      unconfirmedBalanceSat: 0,
      utxos: {},
      fetchedLast: 0,
      used: false,
    };
    expect(unusedAddress).to.deep.equal(expectedUnused);

    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
});
