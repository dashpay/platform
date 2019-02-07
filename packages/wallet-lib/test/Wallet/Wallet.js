const { expect } = require('chai');
const knifeMnemonic = require('../fixtures/knifeeasily');
const fluidMnemonic = require('../fixtures/fluidDepth');
const cR4t6ePrivateKey = require('../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../src/CONSTANTS');
const { Wallet } = require('../../src');
const inMem = require('../../src/adapters/InMem');
const Dashcore = require('@dashevo/dashcore-lib');

const mocks = {
  adapter: inMem,
  offlineMode: true,
};
describe('Wallet - class', () => {
  it('should create a wallet without parameters', () => {
    const wallet1 = new Wallet(mocks);
    expect(wallet1.type).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet1.mnemonic).toString()).to.be.equal(wallet1.mnemonic);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.keyChain.type).to.be.deep.equal('HDRootKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.length(10);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet);

    const wallet2 = new Wallet(mocks);
    expect(wallet2.type).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet2.mnemonic).toString()).to.be.equal(wallet2.mnemonic);
    expect(wallet2.mnemonic).to.be.not.equal(wallet1.mnemonic);
    expect(wallet2.network).to.be.deep.equal(Dashcore.Networks.testnet);

    wallet1.storage.events.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
    wallet2.storage.events.on('CONFIGURED', () => {
      wallet2.disconnect();
    });
  });
  it('should create a wallet with mnemonic', () => {
    const wallet1 = new Wallet(Object.assign({ mnemonic: knifeMnemonic.mnemonic }, mocks));
    expect(wallet1.type).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(Dashcore.Mnemonic(wallet1.mnemonic).toString()).to.be.equal(wallet1.mnemonic);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet);
    expect(wallet1.keyChain.type).to.be.deep.equal('HDRootKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(knifeMnemonic.walletIdTestnet);

    const opts2 = Object.assign({ mnemonic: knifeMnemonic.mnemonic, network: 'livenet' }, mocks);
    const wallet2 = new Wallet(opts2);
    expect(wallet2.type).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(wallet2.network).to.be.deep.equal(Dashcore.Networks.mainnet);
    expect(Dashcore.Mnemonic(wallet2.mnemonic).toString()).to.be.equal(wallet2.mnemonic);
    expect(wallet2.walletId).to.be.equal(knifeMnemonic.walletIdMainnet);
    wallet1.storage.events.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
    wallet2.storage.events.on('CONFIGURED', () => {
      wallet2.disconnect();
    });
  });
  it('should create a wallet with HDPrivateKey', () => {
    const wallet1 = new Wallet(Object.assign({ seed: knifeMnemonic.HDRootPrivateKeyTestnet, network: 'testnet' }, mocks));
    expect(wallet1.type).to.be.equal(WALLET_TYPES.HDWALLET);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet);
    expect(wallet1.keyChain.type).to.be.deep.equal('HDRootKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(knifeMnemonic.walletIdTestnet);
    wallet1.storage.events.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
  it('should create a wallet with PrivateKey', () => {
    const wallet1 = new Wallet(Object.assign({ privateKey: cR4t6ePrivateKey.privateKey, network: 'testnet' }, mocks));
    expect(wallet1.type).to.be.equal(WALLET_TYPES.SINGLE_ADDRESS);
    expect(wallet1.mnemonic).to.be.equal(null);

    expect(wallet1.plugins).to.be.deep.equal({});
    expect(wallet1.accounts).to.be.deep.equal([]);
    expect(wallet1.network).to.be.deep.equal(Dashcore.Networks.testnet);
    expect(wallet1.keyChain.type).to.be.deep.equal('privateKey');
    expect(wallet1.passphrase).to.be.deep.equal(null);
    expect(wallet1.allowSensitiveOperations).to.be.deep.equal(false);
    expect(wallet1.injectDefaultPlugins).to.be.deep.equal(true);
    expect(wallet1.walletId).to.be.equal(cR4t6ePrivateKey.walletIdTestnet);

    wallet1.storage.events.on('CONFIGURED', () => {
      wallet1.disconnect();
    });
  });
});
describe('Wallet - Get/Create Account', () => {
  const wallet1 = new Wallet(Object.assign({ mnemonic: fluidMnemonic.mnemonic }, mocks));

  it('should be able to create/get a wallet', () => {
    const acc1 = wallet1.createAccount({ injectDefaultPlugins: false });
    const acc2 = wallet1.createAccount({ injectDefaultPlugins: false });
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
  it('should get an account in a wallet', () => {
    const acc1 = wallet1.getAccount({ index: 0 });
    const acc2 = wallet1.getAccount({ index: 1 });

    expect(acc1).to.be.deep.equal(wallet1.getAccount());

    [acc1, acc2].forEach((el, i) => {
      // eslint-disable-next-line no-unused-expressions
      expect(el).to.exist;
      expect(el).to.be.a('object');
      expect(el.constructor.name).to.equal('Account');
      expect(el.BIP44PATH).to.equal(`m/44'/1'/${i}'`);
    });
    wallet1.disconnect();
  });
  it('should encrypt wallet with a passphrase', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: fluidMnemonic.mnemonic,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(Object.assign(config, mocks));
    const encryptedHDPriv = walletTestnet.exportWallet(true);
    const expectedHDPriv = 'tprv8ZgxMBicQKsPcuZMDBeTL2qaBF7gyUPt2wbqbJG2yp8s7yzRE1cRcjRnG3Xmdv3sELwtLGz186VX3EeHQ5we1xr1qH95QN6FRopP6FZqBUJ';
    expect(encryptedHDPriv.toString()).to.equal(expectedHDPriv);
    walletTestnet.storage.events.on('CONFIGURED', () => {
      walletTestnet.disconnect();
    });
  });
  it('should be able to create an account at a specific index', () => {
    const network = Dashcore.Networks.testnet;
    const passphrase = 'Evolution';
    const config = {
      mnemonic: fluidMnemonic.mnemonic,
      passphrase,
      network,
    };
    const walletTestnet = new Wallet(Object.assign(config, mocks));

    const account = walletTestnet.createAccount();
    // eslint-disable-next-line no-unused-expressions
    expect(account).to.exist;
    expect(account.BIP44PATH.split('/')[3]).to.equal('0\'');
    expect(account.accountIndex).to.equal(0);


    const accountSpecificIndex = walletTestnet.createAccount({ accountIndex: 42 });
    expect(accountSpecificIndex.BIP44PATH.split('/')[3]).to.equal('42\'');
    expect(accountSpecificIndex.accountIndex).to.equal(42);
    walletTestnet.storage.events.on('CONFIGURED', () => {
      walletTestnet.disconnect();
    });
  });
  it('should stop the wallet', () => {

  });
});
