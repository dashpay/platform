const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const knifeMnemonic = require('../../../fixtures/knifeeasily');
const gatherSailMnemonic = require('../../../fixtures/gathersail');
const fluidMnemonic = require('../../../fixtures/fluidDepth');
const cR4t6ePrivateKey = require('../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { Wallet } = require('../../index');
const inMem = require('../../adapters/InMem');
const fromHDPublicKey = require('./methods/fromHDPublicKey');
const gatherSail = require('../../../fixtures/gathersail');

const mocks = {
  adapter: inMem,
  offlineMode: true,
};
describe('Wallet - class', function suite() {
  this.timeout(10000);
  it('should create a wallet without parameters', () => {
    const wallet1 = new Wallet(mocks);
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet1.mnemonic).toString()).to.be.equal(wallet1.mnemonic);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.keyChain.type).to.be.deep.equal('HDPrivateKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.length(10);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());

    const wallet2 = new Wallet(mocks);
    expect(wallet2.walletType).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet2.mnemonic).toString()).to.be.equal(wallet2.mnemonic);
    expect(wallet2.mnemonic).to.be.not.equal(wallet1.mnemonic);
    expect(wallet2.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
    wallet2.storage.on('CONFIGURED', () => {
      wallet2.disconnect();
    });
  });
  it('should create a wallet with mnemonic', () => {
    const wallet1 = new Wallet({ mnemonic: knifeMnemonic.mnemonic, ...mocks });
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet1.mnemonic).toString()).to.be.equal(wallet1.mnemonic);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('HDPrivateKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(knifeMnemonic.walletIdTestnet);

    const opts2 = { mnemonic: knifeMnemonic.mnemonic, network: 'livenet', ...mocks };
    const wallet2 = new Wallet(opts2);
    expect(wallet2.walletType).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(wallet2.network).to.be.deep.equal(Dashcore.Networks.mainnet.toString());
    expect(Dashcore.Mnemonic(wallet2.mnemonic).toString()).to.be.equal(wallet2.mnemonic);
    expect(wallet2.walletId).to.be.equal(knifeMnemonic.walletIdMainnet);
    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
    wallet2.storage.on('CONFIGURED', () => {
      wallet2.disconnect();
    });
  });
  it('should create a wallet with HDPrivateKey', () => {
    const wallet1 = new Wallet({ HDPrivateKey: knifeMnemonic.HDRootPrivateKeyTestnet, network: 'testnet', ...mocks });
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('HDPrivateKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(knifeMnemonic.walletIdTestnet);
    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });

  it('should create a wallet with HDPublicKey', () => {
    const wallet1 = new Wallet({ HDPublicKey: gatherSailMnemonic.testnet.external.hdpubkey, network: 'testnet', ...mocks });
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.HDPUBLIC);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('HDPublicKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(gatherSailMnemonic.testnet.external.walletId);
    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
  it('should create a wallet with PrivateKey', () => {
    const wallet1 = new Wallet({ privateKey: cR4t6ePrivateKey.privateKey, network: 'testnet', ...mocks });
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.PRIVATEKEY);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('privateKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(cR4t6ePrivateKey.walletIdTestnet);

    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
  it('should create a wallet with PublicKey', () => {
    const publicKey = new Dashcore.PrivateKey(cR4t6ePrivateKey.privateKey).toPublicKey();
    expect(publicKey.toString()).to.equal('03353b4deb77923b026278d116e2007d6f97a058e42d35f1fd39efd5314705f844');
    const wallet1 = new Wallet({ publicKey: publicKey.toString(), network: 'testnet', ...mocks });
    expect(wallet1.walletType).to.be.equal(WALLET_TYPES.PUBLICKEY);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet1.keyChain.type).to.be.deep.equal('publicKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal('9f1f6f37f7');

    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });

    const wallet2 = new Wallet({ publicKey, network: 'testnet', ...mocks });
    expect(wallet2.walletType).to.be.equal(WALLET_TYPES.PUBLICKEY);
    expect(wallet2.mnemonic).to.be.equal(null);

    expect(wallet2.plugins).to.be.deep.equal({});
    expect(wallet2.accounts).to.be.deep.equal([]);
    expect(wallet2.network).to.be.deep.equal(Dashcore.Networks.testnet.toString());
    expect(wallet2.keyChain.type).to.be.deep.equal('publicKey');
    expect(wallet2.passphrase).to.be.deep.equal(null);
    expect(wallet2.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet2.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet2.walletId).to.be.equal('9f1f6f37f7');

    wallet2.storage.on('CONFIGURED', () => {
      wallet2.disconnect();
    });
  });
  it('should have an offline Mode', () => {
    const wallet = new Wallet({
      offlineMode: true, privateKey: cR4t6ePrivateKey.privateKey, network: 'testnet', ...mocks,
    });
    expect(wallet.offlineMode).to.equal(true);
    wallet.storage.on('CONFIGURED', () => {
      wallet.disconnect();
    });
  });
});
describe('Wallet - Get/Create Account',  function suite() {
  this.timeout(10000);
  const wallet1 = new Wallet({ mnemonic: fluidMnemonic.mnemonic, ...mocks });

  it('should be able to create/get a wallet', async () => {
    const acc1 = await wallet1.createAccount({ injectDefaultPlugins: false });
    const acc2 = await wallet1.createAccount({ injectDefaultPlugins: false });

    [acc1, acc2].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    acc1.disconnect();
    acc2.disconnect();
  });
  it('should get an account in a wallet', async () => {
    const acc1 = await wallet1.getAccount({ index: 0 });
    const acc2 = await wallet1.getAccount({ index: 1 });

    expect(acc1).to.be.deep.equal(await wallet1.getAccount());

    [acc1, acc2].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    wallet1.storage.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
  it('should encrypt wallet with a passphrase', () => {
    const network = Dashcore.Networks.testnet.toString();
    const passphrase = 'Evolution';
    const config = {
      mnemonic: fluidMnemonic.mnemonic,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(Object.assign(config, mocks));
    const encryptedHDPriv = walletTestnet.exportWallet('HDPrivateKey');
    const expectedHDPriv = 'tprv8ZgxMBicQKsPcuZMDBeTL2qaBF7gyUPt2wbqbJG2yp8s7yzRE1cRcjRnG3Xmdv3sELwtLGz186VX3EeHQ5we1xr1qH95QN6FRopP6FZqBUJ';
    expect(encryptedHDPriv.toString()).to.equal(expectedHDPriv);
    walletTestnet.storage.on('CONFIGURED', () => {
      walletTestnet.disconnect();
    });
  });
  it('should be able to create an account at a specific index', (done) => {
    const network = Dashcore.Networks.testnet.toString();
    const passphrase = 'Evolution';
    const config = {
      mnemonic: fluidMnemonic.mnemonic,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(Object.assign(config, mocks));

    walletTestnet.createAccount().then(async (account)=>{
      // eslint-disable-next-line no-unused-expressions
      expect(account).to.exist;
      expect(account.BIP44PATH.split('/')[3]).to.equal('0\'');
      expect(account.index).to.equal(0);

      const accountSpecificIndex = await walletTestnet.createAccount({ index: 42 });
      expect(accountSpecificIndex.BIP44PATH.split('/')[3]).to.equal('42\'');
      expect(accountSpecificIndex.index).to.equal(42);
      walletTestnet.disconnect();
      done();
    })
  });
  it('should not leak', () => {
    const mockOpts1 = { };
    fromHDPublicKey.call(mockOpts1, gatherSail.testnet.external.hdpubkey);
    expect(mockOpts1.keyChain.keys).to.deep.equal({});
  });
});
