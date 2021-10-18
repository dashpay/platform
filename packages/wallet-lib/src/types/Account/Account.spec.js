const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const knifeMnemonic = require('../../../fixtures/knifeeasily');
const fluidMnemonic = require('../../../fixtures/fluidDepth');
const cR4t6ePrivateKey = require('../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { Account, EVENTS } = require('../../index');
const EventEmitter = require('events');
const inMem = require('../../adapters/InMem');
const blockHeader = new Dashcore.BlockHeader.fromObject({
  hash: '00000ac3a0c9df709260e41290d6902e5a4a073099f11fe8c1ce80aadc4bb331',
  version: 2,
  prevHash: '00000ce430de949c85a145b02e33ebbaed3772dc8f3d668f66edc6852c24d002',
  merkleRoot: '663360403b5fba9cd8744c3706f9660c7d3fee4e5a9ee98ce0ad5e5ad7824c1d',
  time: 1398712821,
  bits: 504365040,
  nonce: 312363
});
const mocks = {
  adapter: inMem,
  offlineMode: true,
};


describe('Account - class', function suite() {
  this.timeout(10000);
  before(() => {
    const emitter = new EventEmitter();
    const mockStorage = {
      on: emitter.on,
      emit: emitter.emit,
      store: {},
      getStore: () => {},
      saveState: () => {},
      stopWorker: () => {},
      createAccount: () => {},
      importBlockHeader: (blockheader)=>{
        mockStorage.emit(EVENTS.BLOCKHEADER, {type: EVENTS.BLOCKHEADER, payload:blockheader});
      }
    };
    mocks.wallet = (new (function Wallet() {
      this.walletId = '1234567891';
      this.walletType =  WALLET_TYPES.HDWALLET;
      this.accounts = [];
      this.network = Dashcore.Networks.testnet;
      this.storage = mockStorage;
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
  it('should forward events', function (done) {
    const mockWallet = mocks.wallet;
    const account = new Account(mockWallet, { injectDefaultPlugins: false });
    account.init(mockWallet).then(async ()=>{
      await account.on(EVENTS.BLOCKHEADER, ()=>{
        done();
      });
      account.storage.importBlockHeader(blockHeader);
    })

  });
});
