const { expect } = require('chai');
const { Wallet } = require('../src/index');
const { Account } = require('../src/index');
const { mnemonicString1, mnemonicString2 } = require('./fixtures.json');

const fixtures = {
  increasetable: require('./fixtures/increasetable'),
  sunnysoccer: require('./fixtures/sunnysoccer'),
};

const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');
const InsightClient = require('../src/transports/Insight/insightClient');

// const seed1 = new Mnemonic(mnemonic1).toSeed();
// const dapiClient = 'placeholder';
// const network = 'testnet';
// const privateHDKey1 = new Dashcore.HDPrivateKey.fromSeed(seed1, Dashcore.Networks.testnet);
const walletConfigs = {
  testnet: {
    network: 'testnet',
    mnemonic: mnemonicString1,
  },
  livenet: {
    network: 'livenet',
    mnemonic: mnemonicString1,
  },
  fakeTransportTestnet: {
    network: 'testnet',
    mnemonic: fixtures.increasetable.mnemonic,
  },
  fakeTransportLivenet: {
    network: 'livenet',
    mnemonic: fixtures.increasetable.mnemonic,
    cache: {
      addresses: fixtures.increasetable.addresses.external,
    },
  },
  fakeTransportWithUTXOTestnet: {
    network: 'testnet',
    mnemonic: fixtures.increasetable.mnemonic,
    cache: {
      addresses: fixtures.increasetable.addresses.external,
      transactions: fixtures.increasetable.transactions,
    },
  },
  fakeTransportWithNonContiguousCacheTestnet: {
    network: 'testnet',
    mnemonic: fixtures.sunnysoccer.mnemonic,
    cache: {
      addresses: fixtures.sunnysoccer.addresses.external,
    },
  },
};

describe('Account - Basics', function suite() {
  this.timeout(5000);
  it('should be able to create an Account without any params', () => {
    const wallet = new Wallet();
    const account = wallet.createAccount();
    expect(account).to.exist;
    expect(account).to.be.a('object');
    expect(account.constructor.name).to.equal('Account');
    expect(account.accountIndex).to.equal(0);
    expect(account.BIP44PATH).to.equal('m/44\'/1\'/0\'');
    expect(account.store.wallets).to.exist;

    const walletId = wallet.walletId;
    expect(account.store.wallets[walletId].accounts["m/44'/1'/0'"]).to.exist;
    expect(account.transport).to.not.equal(null);
    expect(account.mode).to.equal('full');
    expect(account.label).to.equal(null);
    expect(account.cacheTx).to.equal(true);
    expect(account.workers).to.be.a('object');
    expect(account.workers.bip44).to.be.a('object');

    wallet.disconnect();
  });
  it('should be able to create an account with basic parameters', () => {
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.increasetable.mnemonic,
    });
    const account = wallet.createAccount({ mode: 'full' });
    expect(account).to.exist;
    expect(account).to.be.a('object');
    expect(account.constructor.name).to.equal('Account');
    expect(account.accountIndex).to.equal(0);
    expect(account.BIP44PATH).to.equal('m/44\'/1\'/0\'');
    expect(account.transport).to.not.equal(null);
    expect(account.mode).to.equal('full');
    expect(account.label).to.equal(null);
    expect(account.cacheTx).to.equal(true);
    expect(account.workers).to.be.a('object');
    expect(account.workers.bip44).to.be.a('object');
    wallet.disconnect();
  });
  it('should be able to create an account with cache parameters', () => {
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.increasetable.mnemonic,
    });
    const account = wallet.createAccount({
      cache: {
        addresses: fixtures.increasetable.addresses.external,
      },
    });
    expect(account).to.exist;
    expect(account).to.be.a('object');
    expect(account.constructor.name).to.equal('Account');
    expect(account.accountIndex).to.equal(0);
    expect(account.BIP44PATH).to.equal('m/44\'/1\'/0\'');
    expect(account.transport).to.not.equal(null);
    expect(account.mode).to.equal('full');
    expect(account.label).to.equal(null);
    expect(account.cacheTx).to.equal(true);
    expect(account.workers).to.be.a('object');
    expect(account.workers.bip44).to.be.a('object');
    expect(account.getAddresses()).to.deep.equal(fixtures.increasetable.addresses.external);
    wallet.disconnect();
  });

  it('should be able to cache an account with transactions', () => {
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.increasetable.mnemonic,
    });
    const opts = { cache: { transactions: fixtures.increasetable.transactions } };
    const account = wallet.createAccount(opts);
    expect(account).to.exist;
    expect(account).to.be.a('object');
    expect(account.constructor.name).to.equal('Account');
    expect(account.accountIndex).to.equal(0);
    expect(account.BIP44PATH).to.equal('m/44\'/1\'/0\'');
    expect(account.transport).to.not.equal(null);
    expect(account.mode).to.equal('full');
    expect(account.label).to.equal(null);
    expect(account.cacheTx).to.equal(true);
    expect(account.workers).to.be.a('object');
    expect(account.workers.bip44).to.be.a('object');
    expect(account.getTransactions()).to.deep.equal(fixtures.increasetable.transactions);

    wallet.disconnect();
  });
  it('should warn on invalid structure', () => {
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.increasetable.mnemonic,
    });
    const invalidOpts = { cache: { transactions: fixtures.increasetable.invalidTransactions } };
    expect(() => wallet.createAccount(invalidOpts)).to.throw("Can't import this transaction. Invalid structure.");
    wallet.disconnect();
  });
  it('should not forgot to generate missing (gap) addresse due to cache import', () => {
    const expected = fixtures.sunnysoccer.getAddresses;
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.sunnysoccer.mnemonic,
      cache: {
        addresses: fixtures.sunnysoccer.getAddresses,
      },
    });
    const account = wallet.createAccount();
    const result = account.getAddresses();
    expect(result).to.deep.equal(expected);
    wallet.disconnect();
  });
  it('should be able to setup a label', () => {
    const wallet = new Wallet();
    const label = 'MyUberLabel';
    const account = wallet.createAccount({
      label,
      mode: 'light',
    });
    expect(account.label).to.equal(label);
    wallet.disconnect();
  });
  it('should not be able to create an account without wallet', () => {
    expect(() => new Account()).to.throw('Expected wallet to be created and passed as param');
  });
  it('should be able to create an additionnal account', () => {
    const wallet = new Wallet({
      network: 'testnet',
      mnemonic: fixtures.increasetable.mnemonic,
    });
    wallet.createAccount({ mode: 'light' });
    const account = wallet.createAccount({ mode: 'light' });
    // eslint-disable-next-line no-unused-expressions
    expect(account).to.exist;
    expect(account).to.be.a('object');
    expect(account.constructor.name).to.equal('Account');
    expect(account.BIP44PATH).to.equal('m/44\'/1\'/1\'');
    wallet.disconnect();
  });

  it('should create an account using livenet network', () => {
    const wallet = new Wallet({ network: 'livenet' });
    const accountLivenet = wallet.createAccount({ mode: 'light' }); // Should derivate
    // eslint-disable-next-line no-unused-expressions
    expect(accountLivenet).to.exist;
    expect(accountLivenet).to.be.a('object');
    expect(accountLivenet.constructor.name).to.equal('Account');
    expect(accountLivenet.BIP44PATH).to.equal('m/44\'/5\'/0\'');
    wallet.disconnect();
  });
  it('should not be able to broadcast Transaction without transport', () => {
    const wallet = new Wallet(walletConfigs.fakeTransportWithUTXOTestnet);
    const account = wallet.createAccount();
    account.events.on('ready', async () => {
      const options = {
        to: 'yiUjSkhkAfaHfYYmTMhc27NCmogJ3iRBaS',
        satoshis: 100000,
        isInstantSend: false,
      };
      const rawtx = account.createTransaction(options);
      return account.broadcastTransaction(rawtx)
        .then(
          () => Promise.reject(new Error('Expected method to reject.')),
          err => expect(err).to.be.a('Error').with.property('message', 'A transport layer is needed to perform a broadcast'),
        ).then(wallet.disconnect.bind(wallet));
    });
  });


  it('should not be able to broadcast Transaction with invalid transport', () => {
    const wallet = new Wallet(walletConfigs.fakeTransportWithUTXOTestnet);
    const account = wallet.createAccount();
    const options = {
      to: 'yiUjSkhkAfaHfYYmTMhc27NCmogJ3iRBaS',
      satoshis: 100000,
      isInstantSend: false,
      transport: 'fake',
    };

    const rawtx = account.createTransaction(options);
    return account.broadcastTransaction(rawtx)
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', 'A transport layer is needed to perform a broadcast'),
      ).then(wallet.disconnect.bind(wallet));
  });
});

const walletFakeTransportWithUTXO = new Wallet(walletConfigs.fakeTransportWithUTXOTestnet);
const walletFakeTransportLivenet = new Wallet(walletConfigs.fakeTransportLivenet);
let accountFakeTransportWithUTXO;
let accountFakeTransportLivenet;

describe('Account - Transports, Workers', function suite() {
  this.timeout(5000);
  before((done) => {
    const accountsReadinessWatcher = {
      accountFakeTransportWithUTXO: { ready: false },
      accountFakeTransportLivenet: { ready: false },
      interval: null,
      clearInterval() {
        clearInterval(this.interval);
      },
      isReadyYet() {
        return this.accountFakeTransportWithUTXO.ready && this.accountFakeTransportLivenet.ready;
      },
    };
    accountFakeTransportWithUTXO = walletFakeTransportWithUTXO.createAccount();
    accountFakeTransportWithUTXO.events.on('ready', () => (accountsReadinessWatcher.accountFakeTransportWithUTXO.ready = true));

    accountFakeTransportLivenet = walletFakeTransportLivenet.createAccount();
    accountFakeTransportLivenet.events.on('ready', () => (accountsReadinessWatcher.accountFakeTransportLivenet.ready = true));

    accountsReadinessWatcher.interval = setInterval(() => {
      if (accountsReadinessWatcher.isReadyYet()) {
        accountsReadinessWatcher.clearInterval();
        done();
      }
    }, 20);
  });
  it('should be able to derivate an address for livenet', () => {
    const account = accountFakeTransportLivenet;
    const addressData = account.getAddress(0, true);
    expect(addressData).to.have.property('address');
    const { address } = addressData;
    expect(address).to.equal('Xc1u8mcXcRm7GAHtFvYXYnPR8L6cqVkoyp');

    const addressDataInternal = account.getAddress(0, false);
    expect(addressDataInternal.address).to.equal('XcqD6TFCcQ7jvcNiuohFFxBmwP2n71tANy');

    const addressDataExternal = account.getAddress(10);
    expect(addressDataExternal.address).to.equal('XfPKRskw2vDXKpJ11oZZdQBjSMocJ5jmes');
  });
  it('should be able to derivate an address for testnet', () => {
    const account = accountFakeTransportWithUTXO;
    const addressData = account.getAddress(0, true);
    expect(addressData).to.have.property('address');
    const { address, path } = addressData;
    expect(address).to.equal('yizmJb63ygipuJaRgYtpWCV2erQodmaZt8');
    expect(path).to.equal('m/44\'/1\'/0\'/0/0');

    const addressDataInternal = account.getAddress(0, false);
    expect(addressDataInternal.address).to.equal('yjSivd8eWH1vVywaeePiHBLXqMbHFXxxXE');
    expect(addressDataInternal.path).to.equal('m/44\'/1\'/0\'/1/0');

    const addressDataExternal = account.getAddress(10);
    expect(addressDataExternal.address).to.equal('yPetUJo1WeAjLpNYACntNSgEvHuUu3p1a8');
    expect(addressDataExternal.path).to.equal('m/44\'/1\'/0\'/0/10');
  });
  it('should be able to get all address', () => {
    const account = accountFakeTransportWithUTXO;
    const addressesExternalData = account.getAddresses();
    const externalDataKeys = Object.keys(addressesExternalData);
    expect(externalDataKeys.length).to.equal(21);// 20 unused + 1 used

    const addressesInternalData = account.getAddresses(false);
    const internalDataKeys = Object.keys(addressesInternalData);
    expect(internalDataKeys.length).to.equal(21);
  });

  it('should be able to get a unused address', () => {
    const account = accountFakeTransportWithUTXO;
    const unusedAddress = account.getUnusedAddress();
    expect(unusedAddress.path).to.equal('m/44\'/1\'/0\'/0/1');
    expect(unusedAddress.index).to.equal('1');// TODO : why would index even be a string...
    expect(unusedAddress.address).to.equal('yhLGmtf5Jmdb3DUvsaNJUHyCjjxTcBJEry');
  });
  it('should be able to fetch UTXO from an amount', () => {
    const account = accountFakeTransportWithUTXO;
    const utxos = account.getUTXOS();
    const expectedUtxos = [
      {
        address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt8',
        outputIndex: 0,
        satoshis: 100000000,
        script: '76a914f8c2652847720ab6d401291e5a48e2c8fe5d3c9f88ac',
        txId: 'dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc',
      },
    ];
    expect(utxos).to.deep.equal(expectedUtxos);
    // Fetch also unavailable utxos
    const allUtxos = account.getUTXOS(false);
    expect(allUtxos).to.deep.equal(expectedUtxos);
  });


  it('should not be able to generate an address without path', () => {
    const account = accountFakeTransportWithUTXO;
    expect(() => account.generateAddress()).to.throw('Expected path to generate an address');
  });


  it('should not be able to sign without parameters', () => {
    const account = accountFakeTransportWithUTXO;
    expect(() => account.sign()).to.throw('Require one or multiple privateKeys to sign');
  });

  it('should not be able to sign when nothing to sign', () => {
    const account = accountFakeTransportWithUTXO;
    const privateKeys = account.getPrivateKeys(account.getUTXOS().map(el => ((el.address))));
    expect(() => account.sign('', privateKeys)).to.throw('Nothing to sign');
  });

  it('should not be able to sign when invalid object type', () => {
    const account = accountFakeTransportWithUTXO;
    const privateKeys = account.getPrivateKeys(account.getUTXOS().map(el => ((el.address))));
    expect(() => account.sign([1, 1, 2], privateKeys)).to.throw('Unhandled object of type Array');
  });
  it('should not be able to sign when invalid object type', () => {
    const account = accountFakeTransportWithUTXO;
    const privateKeys = account.getPrivateKeys(account.getUTXOS().map(el => ((el.address))));
    expect(() => account.sign([1, 1, 2], privateKeys)).to.throw('Unhandled object of type Array');
  });
  it('should allow to refresh an account without a transport', () => {
    const account = accountFakeTransportWithUTXO;
    expect(account.forceRefreshAccount()).to.equal(true);
  });
  it('should detect lack of recipient', () => {
    const account = accountFakeTransportWithUTXO;
    const options = {
      satoshis: 1000000000,
      isInstantSend: false,
    };
    expect(() => {
      account.createTransaction(options);
    }).to.throw('A recipient is expected to create a transaction');
  });
  it('should detect lack of amount/satoshi', () => {
    const account = accountFakeTransportWithUTXO;
    const options = {
      to: 'yiUjSkhkAfaHfYYmTMhc27NCmogJ3iRBaS',
      isInstantSend: false,
    };
    expect(() => {
      account.createTransaction(options);
    }).to.throw('An amount in dash or in satoshis is expected to create a transaction');
  });
  it('should fire an error if we try to create a tx with more fund than in balance', () => {
    const account = accountFakeTransportWithUTXO;
    const options = {
      to: 'yiUjSkhkAfaHfYYmTMhc27NCmogJ3iRBaS',
      satoshis: 1000000000,
      isInstantSend: false,
    };
    expect(() => {
      account.createTransaction(options);
    }).to.throw('Not enought fund');
  });
  it('should get a balance of an account', () => {
    const account = accountFakeTransportWithUTXO;
    expect(account.getBalance()).to.equal(100000000);
    expect(account.getBalance(true)).to.equal(100000000);
    expect(account.getBalance(true, false)).to.equal(1);
  });
  it('should get transaction history of an account', () => {
    const account = accountFakeTransportWithUTXO;
    return account
      .getTransactionHistory()
      .then((history) => {
        const expected = fixtures.increasetable.getHistory;
        expect(history).to.deep.equal(expected);
      });
  });
  it('should be able to switch the network', () => {
    const account = accountFakeTransportWithUTXO;
    expect(account.network.toString()).to.equal('testnet');
    account.updateNetwork('livenet');
    expect(account.network.toString()).to.equal('livenet');
    const change = account.updateNetwork('something');
    expect(change).to.equal(false);
    expect(account.network.toString()).to.equal('testnet');
  });
  it('should not be able to fetchStatus with invalid transport layer', () => {
    const account = accountFakeTransportWithUTXO;
    return account.fetchStatus()
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', 'A transport layer is needed to fetch status'),
      );
  });
  it('should not be able to fetchAddressInfo with invalid transport layer', () => {
    const account = accountFakeTransportWithUTXO;
    return account.fetchAddressInfo()
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', 'A transport layer is needed to fetch addr info'),
      );
  });
  it('should not be able to fetchTransactionInfo with invalid transport layer', () => {
    const account = accountFakeTransportWithUTXO;
    return account.fetchTransactionInfo()
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', 'A transport layer is needed to fetch tx info'),
      );
  });
  after(() => {
    walletFakeTransportWithUTXO.disconnect();
    accountFakeTransportLivenet.disconnect();
  });
});
