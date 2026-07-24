const EventEmitter = require('events');
const { expect } = require('chai');
const DAPIClient = require('@dashevo/dapi-client');

const Wallet = require('../Wallet');
const EVENTS = require('../../../EVENTS');
const logger = require('../../../logger');
const BlockHeadersSyncWorker = require('../../../plugins/Workers/BlockHeadersSyncWorker/BlockHeadersSyncWorker');
const { mineHeadersChain } = require('../../../test/mocks/dashcore/block');

const { BlockHeadersProvider } = DAPIClient;

class TestTransport {
  constructor(blockHeadersProvider) {
    this.client = {
      blockHeadersProvider,
    };
  }
}

class TestTransportWithoutClient {}

const waitForStorage = async (wallet) => {
  if (!wallet.storage.configured) {
    await new Promise((resolve) => {
      wallet.storage.once(EVENTS.CONFIGURED, resolve);
    });
  }
};

describe('Wallet#createAccount', function suite() {
  let wallet;

  afterEach(async () => {
    if (wallet) {
      await wallet.disconnect();
    }
  });

  it('should wait for genesis initialization when the online store has no headers', async function shouldInitializeEmptyStore() {
    let resolveInitialization;
    const initializationPromise = new Promise((resolve) => {
      resolveInitialization = resolve;
    });
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().returns(initializationPromise),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });

    let accountCreated = false;
    const createAccountPromise = wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    }).then((account) => {
      accountCreated = true;
      return account;
    });

    await new Promise((resolve) => {
      setTimeout(resolve, 0);
    });

    expect(blockHeadersProvider.initializeChainWith)
      .to.have.been.calledOnceWithExactly([], -1);
    expect(accountCreated).to.equal(false);

    resolveInitialization();
    await createAccountPromise;
  });

  it('should resume from the first authenticated stored header without a remote checkpoint', async function shouldInitializeStoredHeaders() {
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);
    const chainStore = wallet.storage.getDefaultChainStore();
    const storedHeaders = [
      { hash: 'stored-header-40' },
      { hash: 'stored-header-41' },
      { hash: 'stored-header-42' },
    ];
    chainStore.state.blockHeaders = storedHeaders;
    chainStore.state.lastSyncedHeaderHeight = 42;

    await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(blockHeadersProvider.initializeChainWith)
      .to.have.been.calledOnceWithExactly(storedHeaders, 40);
  });

  it('should normalize a stale one-header context before genesis synchronization', async function shouldNormalizeInsufficientHeaders() {
    const blockHeadersProvider = new BlockHeadersProvider({ network: 'regtest' });
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    const [storedHeader] = await mineHeadersChain('regtest', 1);
    const transactions = new Map();
    chainStore.state.chainHeight = 42;
    chainStore.state.lastSyncedBlockHeight = 40;
    chainStore.state.blockHeaders = [storedHeader];
    chainStore.state.lastSyncedHeaderHeight = 41;
    chainStore.state.headersMetadata = new Map([[storedHeader.hash, { height: 41 }]]);
    chainStore.state.hashesByHeight = new Map([[41, storedHeader.hash]]);
    chainStore.state.transactions = transactions;
    const saveState = this.sinon.spy(wallet.storage, 'saveState');
    const storageKey = `wallet_${wallet.storage.currentWalletId}`;

    await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(saveState).to.have.been.calledOnce();
    expect(chainStore.state.blockHeaders).to.deep.equal([]);
    expect(chainStore.state.lastSyncedHeaderHeight).to.equal(-1);
    expect(chainStore.state.headersMetadata).to.deep.equal(new Map());
    expect(chainStore.state.hashesByHeight).to.deep.equal(new Map());
    expect(chainStore.state.chainHeight).to.equal(42);
    expect(chainStore.state.lastSyncedBlockHeight).to.equal(40);
    expect(chainStore.state.transactions).to.equal(transactions);
    expect(blockHeadersProvider.spvChain.hashesByHeight.has(0)).to.equal(true);

    const normalizedStorage = await wallet.storage.adapter.getItem(storageKey);
    const normalizedChain = normalizedStorage.chains[wallet.storage.currentNetwork];
    expect(normalizedChain.blockHeaders).to.deep.equal([]);
    expect(normalizedChain.lastSyncedHeaderHeight).to.equal(-1);

    const worker = new BlockHeadersSyncWorker({ executeOnStart: false });
    worker.logger = logger;
    worker.storage = wallet.storage;
    worker.transport = wallet.transport;
    worker.parentEvents = new EventEmitter();
    this.sinon.stub(worker, 'scheduleProgressUpdate');

    expect(worker.getStartBlockHeight()).to.equal(1);

    const genesisChain = await mineHeadersChain('regtest', 2);
    blockHeadersProvider.spvChain.addHeaders(genesisChain);
    worker.historicalChainUpdateHandler();
    await wallet.storage.saveState();

    expect(chainStore.state.lastSyncedHeaderHeight).to.equal(1);
    expect(chainStore.state.hashesByHeight.has(0)).to.equal(true);
    expect(chainStore.state.hashesByHeight.has(1)).to.equal(true);

    const rebuiltStorage = await wallet.storage.adapter.getItem(storageKey);
    const rebuiltChain = rebuiltStorage.chains[wallet.storage.currentNetwork];
    expect(rebuiltChain.blockHeaders).to.have.lengthOf(2);
    expect(rebuiltChain.lastSyncedHeaderHeight).to.equal(1);
  });

  it('should initialize and resume from the same minimal stored header context', async function shouldComposeStoredResumeContext() {
    const blockHeadersProvider = new BlockHeadersProvider({ network: 'regtest' });
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const storedHeaders = await mineHeadersChain('regtest', 2);
    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = storedHeaders;
    chainStore.state.lastSyncedHeaderHeight = 42;
    const saveState = this.sinon.stub(wallet.storage, 'saveState').resolves(true);

    await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    const worker = new BlockHeadersSyncWorker({ executeOnStart: false });
    worker.logger = logger;
    worker.storage = wallet.storage;

    expect(saveState).to.have.not.been.called;
    expect(blockHeadersProvider.spvChain.hashesByHeight.has(41)).to.equal(true);
    expect(worker.getStartBlockHeight()).to.equal(42);
  });

  it('should initialize genesis when stored headers imply a negative first height', async function shouldRejectNegativeFirstHeight() {
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = [{ hash: 'header-0' }, { hash: 'header-1' }];
    chainStore.state.lastSyncedHeaderHeight = 0;

    await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(blockHeadersProvider.initializeChainWith)
      .to.have.been.calledOnceWithExactly([], -1);
  });

  it('should reject malformed selected stored headers', async () => {
    const blockHeadersProvider = new BlockHeadersProvider({ network: 'regtest' });
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = [{ malformed: true }, { malformed: true }];
    chainStore.state.lastSyncedHeaderHeight = 42;

    await expect(wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    })).to.be.rejected();

    expect(wallet.accounts).to.deep.equal([]);
  });

  it('should serialize normalization, persistence, and provider initialization', async function shouldSerializeInitialization() {
    let resolveSave;
    const deferredSave = new Promise((resolve) => {
      resolveSave = resolve;
    });
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = [{ hash: 'stored-header-42' }];
    chainStore.state.lastSyncedHeaderHeight = 42;
    chainStore.state.headersMetadata.set('stored-header-42', { height: 42 });
    chainStore.state.hashesByHeight.set(42, 'stored-header-42');
    const saveState = this.sinon.stub(wallet.storage, 'saveState').returns(deferredSave);

    const accountPromises = [
      wallet.createAccount({ injectDefaultPlugins: false, synchronize: false }),
      wallet.createAccount({ injectDefaultPlugins: false, synchronize: false }),
    ];

    await new Promise((resolve) => {
      setTimeout(resolve, 0);
    });

    try {
      expect(chainStore.state.lastSyncedHeaderHeight).to.equal(-1);
      expect(saveState).to.have.been.calledOnce();
      expect(blockHeadersProvider.initializeChainWith).to.have.not.been.called;
    } finally {
      resolveSave();
    }

    await Promise.all(accountPromises);

    expect(blockHeadersProvider.initializeChainWith)
      .to.have.been.calledOnceWithExactly([], -1);
  });

  it('should reject all concurrent accounts when normalized state cannot be saved', async function shouldRejectFailedNormalizationSave() {
    const saveError = new Error('Header state save failed');
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = [{ hash: 'stored-header-42' }];
    chainStore.state.lastSyncedHeaderHeight = 42;
    const saveState = this.sinon.stub(wallet.storage, 'saveState').rejects(saveError);

    const accountPromises = [
      wallet.createAccount({ injectDefaultPlugins: false, synchronize: false }),
      wallet.createAccount({ injectDefaultPlugins: false, synchronize: false }),
    ];

    try {
      const results = await Promise.allSettled(accountPromises);

      expect(results.map(({ status }) => status))
        .to.deep.equal(['rejected', 'rejected']);
      results.forEach(({ reason }) => {
        expect(reason).to.equal(saveError);
      });

      expect(blockHeadersProvider.initializeChainWith).to.have.not.been.called;
      expect(wallet.accounts).to.deep.equal([]);
    } finally {
      saveState.restore();
    }
  });

  it('should normalize in memory without forcing persistence when autoSave is false', async function shouldPreserveDisabledPersistence() {
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      storage: {
        autoSave: false,
      },
      transport: new TestTransport(blockHeadersProvider),
    });
    await waitForStorage(wallet);

    const chainStore = wallet.storage.getDefaultChainStore();
    chainStore.state.blockHeaders = [{ hash: 'stored-header-42' }];
    chainStore.state.lastSyncedHeaderHeight = 42;
    const setItem = this.sinon.spy(wallet.storage.adapter, 'setItem');

    await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(chainStore.state.blockHeaders).to.deep.equal([]);
    expect(chainStore.state.lastSyncedHeaderHeight).to.equal(-1);
    expect(setItem).to.have.not.been.called;
    expect(blockHeadersProvider.initializeChainWith)
      .to.have.been.calledOnceWithExactly([], -1);
  });

  it('should share one provider initialization between concurrent accounts', async function shouldInitializeOnce() {
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().resolves(),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });

    await Promise.all([
      wallet.createAccount({
        injectDefaultPlugins: false,
        synchronize: false,
      }),
      wallet.createAccount({
        injectDefaultPlugins: false,
        synchronize: false,
      }),
    ]);

    expect(blockHeadersProvider.initializeChainWith).to.have.been.calledOnce();
  });

  it('should return one account for concurrent requests of the same index', async function shouldReturnOneAccount() {
    let resolveInitialization;
    const initializationPromise = new Promise((resolve) => {
      resolveInitialization = resolve;
    });
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().returns(initializationPromise),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });

    const accountPromises = [
      wallet.getAccount({
        index: 0,
        injectDefaultPlugins: false,
        synchronize: false,
      }),
      wallet.getAccount({
        index: 0,
        injectDefaultPlugins: false,
        synchronize: false,
      }),
    ];

    await new Promise((resolve) => {
      setTimeout(resolve, 0);
    });
    resolveInitialization();

    const [firstAccount, secondAccount] = await Promise.all(accountPromises);
    expect(firstAccount).to.equal(secondAccount);
    expect(wallet.accounts).to.deep.equal([firstAccount]);
  });

  it('should propagate provider initialization failure without creating an account', async function shouldRejectInitialization() {
    const initializationError = new Error('SPV initialization failed');
    const blockHeadersProvider = {
      initializeChainWith: this.sinon.stub().rejects(initializationError),
    };
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransport(blockHeadersProvider),
    });

    await expect(wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    })).to.be.rejectedWith(initializationError);

    expect(wallet.accounts).to.deep.equal([]);
  });

  it('should preserve online custom transports without a block headers provider', async () => {
    wallet = new Wallet({
      mnemonic: null,
      network: 'regtest',
      transport: new TestTransportWithoutClient(),
    });

    const account = await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(account).to.exist();
    expect(wallet.accounts).to.deep.equal([account]);
  });

  it('should create an offline account without a transport', async () => {
    wallet = new Wallet({
      mnemonic: null,
      offlineMode: true,
    });

    const account = await wallet.createAccount({
      injectDefaultPlugins: false,
      synchronize: false,
    });

    expect(account).to.exist();
    expect(wallet.transport).to.equal(undefined);
  });
});
