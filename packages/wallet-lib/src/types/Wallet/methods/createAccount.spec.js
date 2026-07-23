const { expect } = require('chai');

const Wallet = require('../Wallet');
const EVENTS = require('../../../EVENTS');

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
