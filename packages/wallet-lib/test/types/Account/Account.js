const { expect } = require('chai');
const knifeMnemonic = require('../../fixtures/knifeeasily');
const fluidMnemonic = require('../../fixtures/fluidDepth');
const cR4t6ePrivateKey = require('../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../src/CONSTANTS');
const { Account } = require('../../../src');
const inMem = require('../../../src/adapters/InMem');
const Dashcore = require('@dashevo/dashcore-lib');

const mocks = {
  adapter: inMem,
  offlineMode: true,
};


describe('Account - class', () => {
  before(() => {
    mocks.wallet = (new (function Wallet() {
      this.walletId = '1234567891';
      this.accounts = [];
      this.network = Dashcore.Networks.testnet;
      this.storage = {
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

    expect(account).to.be.deep.equal(mockWallet.accounts[0]);
    expect(account.index).to.be.deep.equal(0);
    expect(account.injectDefaultPlugins).to.be.deep.equal(false);
    expect(account.allowSensitiveOperations).to.be.deep.equal(false);
    expect(account.isReady).to.be.deep.equal(false);
    expect(account.type).to.be.deep.equal(undefined);
    expect(account.transactions).to.be.deep.equal({});
    expect(account.label).to.be.deep.equal(null);
    expect(account.transport).to.be.deep.equal(undefined);
    expect(account.cacheTx).to.be.deep.equal(true);
    expect(account.plugins).to.be.deep.equal({
      workers: {}, DPAs: {}, standard: {}, watchers: {},
    });
    expect(account.readinessInterval.constructor.name).to.be.equal('Timeout');
    expect(account.readinessInterval._idleTimeout).to.be.equal(200);

    account.disconnect();
  });
});
