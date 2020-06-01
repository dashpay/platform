const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const knifeMnemonic = require('../../../fixtures/knifeeasily');
const fluidMnemonic = require('../../../fixtures/fluidDepth');
const cR4t6ePrivateKey = require('../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { Account } = require('../../index');
const inMem = require('../../adapters/InMem');

const mocks = {
  adapter: inMem,
  offlineMode: true,
};


describe('Account - class', function suite() {
  this.timeout(10000);
  before(() => {
    mocks.wallet = (new (function Wallet() {
      this.walletId = '1234567891';
      this.accounts = [];
      this.network = Dashcore.Networks.testnet;
      this.storage = {
        on: () => {},
        emit: () => {},
        store: {},
        getStore: () => {},
        saveState: () => {},
        stopWorker: () => {},
      };
    })());
  });
  it('should be specify on missing params', () => {
    const expectedException1 = 'Expected wallet to be passed as param';
    expect(() => new Account()).to.throw(expectedException1);
  });
  it('should create an account', () => {
    const mockWallet = mocks.wallet;
    const account = new Account(mockWallet, { injectDefaultPlugins: false });
    account.init(mockWallet).then(()=>{
      expect(account).to.be.deep.equal(mockWallet.accounts[0]);
      expect(account.index).to.be.deep.equal(0);
      expect(account.injectDefaultPlugins).to.be.deep.equal(false);
      expect(account.allowSensitiveOperations).to.be.deep.equal(false);
      expect(account.state.isReady).to.be.deep.equal(true);
      expect(account.type).to.be.deep.equal(undefined);
      expect(account.transactions).to.be.deep.equal({});
      expect(account.label).to.be.deep.equal(null);
      expect(account.transport).to.be.deep.equal(undefined);
      expect(account.cacheTx).to.be.deep.equal(true);
      expect(account.plugins).to.be.deep.equal({
        workers: {}, standard: {}, watchers: {},
      });

      account.disconnect();
    })
  });
  it('should correctly create the right expected index', async () => {
    const mockWallet = mocks.wallet;
    const account = new Account(mockWallet, { injectDefaultPlugins: false });
    await account.init(mockWallet);

    const account2 = new Account(mockWallet, { index: 10, injectDefaultPlugins: false });
    await account2.init(mockWallet);

    const account3 = new Account(mockWallet, { injectDefaultPlugins: false });
    await account3.init(mockWallet);

    expect(account.index).to.be.deep.equal(1);
    expect(account2.index).to.be.deep.equal(10);
    expect(account3.index).to.be.deep.equal(2);
    account.disconnect();
    account2.disconnect();
    account3.disconnect();
  });
});
