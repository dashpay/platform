const { expect } = require('chai');
const { Networks } = require('@dashevo/dashcore-lib');
const Storage = require('../src/Storage');
const InMem = require('../src/adapters/InMem');

const adapter = new InMem();
const storageOpts = {
  adapter,
};

const miscFixture = require('./fixtures/misc');

describe('Storage', function suite() {
  this.timeout(50000);
  it('should create a new storage without options', () => {
    const store = new Storage();
    expect(store).to.exist;
    expect(store.store).to.deep.equal({
      wallets: {},
      transactions: {},
    });
    store.stopWorker();
  });
  it('should create a new storage with options', () => {
    const store = new Storage(storageOpts);

    expect(store).to.exist;
    expect(store.adapter).to.deep.equal(adapter);
    expect(store.store).to.deep.equal({
      wallets: {},
      transactions: {},
    });
    expect(store.lastRehydrate).to.equal(null);
    expect(store.lastSave).to.equal(null);
    expect(store.lastModified).to.equal(null);
    expect(store.interval).to.exist;
    store.stopWorker();
  });
  it('should import a tx', () => {
    const store = new Storage(storageOpts);
    const tx = miscFixture['9ab39713e9ce713d41ca6974db83e57bced02402e9516b8a662ed60d5c08f6d1'];

    const result = store.importTransactions(tx);
    expect(result).to.equal(true);
    expect(store.store.transactions[tx.txid]).to.deep.equal(tx);
    store.stopWorker();
  });

  it('should import multiple txs', () => {
    const store = new Storage(storageOpts);
    const tx = miscFixture['1d8f924bef2e24d945d7de2ac66e98c8625e4cefeee4e07db2ea334ce17f9c35'];
    const tx2 = miscFixture['7ae825f4ecccd1e04e6c123e0c55d236c79cd04c6ab64e839aed2ae0af3003e6'];

    expect(() => store.importTransactions([tx, tx2])).to.throw('Not implemented. Please create an issue on github if needed.');
    store.stopWorker();
  });
  it('should importTx deal with wrong type', () => {
    const store = new Storage(storageOpts);
    expect(() => store.importTransactions(12)).to.throw('Invalid transaction. Cannot import.');
    store.stopWorker();
  });
  it('should import a addr', () => {
    const store = new Storage(storageOpts);
    const addr = miscFixture.yizmJb63ygipuJaRgYtpWCV2erQodmaZt8;
    const walletId = 'fad183cbf7';

    const result = store.importAddresses(addr, walletId);
    expect(result).to.equal(true);
    expect(store.store.wallets[walletId]).to.exist;
    expect(store.store.wallets[walletId].addresses.external[addr.path]).to.deep.equal(addr);

    store.stopWorker();
  });
  it('should import multiples addrs', () => {
    const store = new Storage(storageOpts);
    const addrs = {
      "m/44'/1'/0'/0/18": {
        address: '"yTf25xm2t4PeppBpuuGEJktQTYnCaBZ7zE"',
        balanceSat: 0,
        fetchedLast: 1533527600644,
        path: "m/44'/1'/0'/0/18",
        transactions:
        [],
        utxos: [],
      },
      "m/44'/1'/0'/0/19": {
        address: 'yLmv6uX1jmn14pCDpc83YCsA8wHVtcbaNw',
        balanceSat: 0,
        fetchedLast: 1533527600644,
        path: "m/44'/1'/0'/0/19",
        transactions:
        [],
        utxos: [],
      },
      "m/44'/1'/0'/1/0": {
        address: 'yihFsR46sPoFgs43hW652Uw9gm1QmvcWor',
        balanceSat: 0,
        fetchedLast: 1533527600689,
        path: "m/44'/1'/0'/1/0",
        transactions: [],
        utxos: [],
      },
      "m/44'/1'/0'/4/19": {
        address: 'misc',
        balanceSat: 0,
        fetchedLast: 1533527600644,
        path: "m/44'/1'/0'/4/19",
        transactions:
          [],
        utxos: [],
      },


    };
    const walletId = 'fad183cbf7';
    const result = store.importAddresses(addrs, walletId);
    expect(result).to.equal(true);
    const addressStore = store.store.wallets[walletId];
    expect(addressStore.addresses.external['m/44\'/1\'/0\'/0/18']).to.deep.equal(addrs['m/44\'/1\'/0\'/0/18']);
    expect(addressStore.addresses.external['m/44\'/1\'/0\'/0/19']).to.deep.equal(addrs['m/44\'/1\'/0\'/0/19']);


    expect(() => store.importAddresses([], walletId)).to.throw('Not implemented. Please create an issue on github if needed.');

    store.stopWorker();
  });
  it('should import an account', () => {
    const store = new Storage(storageOpts);
    const acc = {
      label: 'uberAcc',
      network: 'testnet',
      path: "m/44'/1'/0'",
    };
    const walletId = 'fad183cbf7';
    const result = store.importAccounts(acc, walletId);
    expect(result).to.equal(true);
    const walletStore = store.store.wallets[walletId];
    expect(walletStore.accounts[acc.path]).to.deep.equal(acc);
    store.stopWorker();
  });
  it('should import multiples account', () => {
    const store = new Storage(storageOpts);
    const accounts = {
      "m/44'/1'/0'": {
        label: 'uberAcc',
        network: 'testnet',
        path: "m/44'/1'/0'",
      },
      "m/44'/1'/1'": {
        label: 'uberAcc2',
        network: 'testnet',
        path: "m/44'/1'/1'",
      },
    };
    const walletId = 'fad183cbf7';

    const result = store.importAccounts(accounts, walletId);
    expect(result).to.equal(true);
    const walletStore = store.store.wallets[walletId];
    expect(walletStore.accounts["m/44'/1'/0'"]).to.deep.equal(accounts["m/44'/1'/0'"]);
    expect(walletStore.accounts["m/44'/1'/1'"]).to.deep.equal(accounts["m/44'/1'/1'"]);
    expect(() => store.importAccounts([])).to.throw('Expected walletId to import addresses');
    expect(() => store.importAccounts([], walletId)).to.throw('Not implemented. Please create an issue on github if needed.');
    expect(() => store.importAccounts(12, walletId)).to.throw('Invalid account');

    store.stopWorker();
  });
  it('should get a store', () => {
    const store = new Storage(storageOpts);
    const acc = {
      label: 'uberAcc',
      network: 'testnet',
      path: "m/44'/1'/0'",
    };
    const walletId = 'fad183cbf7';

    store.importAccounts(acc, walletId);
    const result = store.getStore();

    const expectedResult = {
      transactions: {},
      wallets: {
        fad183cbf7: {
          blockheight: 0,
          mnemonic: null,
          network: Networks.testnet,
          accounts: {
            "m/44'/1'/0'": {
              label: 'uberAcc',
              network: 'testnet',
              path: "m/44'/1'/0'",
            },
          },
          addresses: {
            internal: {},
            external: {},
            misc: {},
          },
        },
      },
    };

    expect(result).to.deep.equal(expectedResult);
    store.stopWorker();
  });
  it('should save a state', (done) => {
    const store = new Storage(storageOpts);
    store.saveState().then((result) => {
      const expectedResult = true;
      expect(result).to.equal(expectedResult);
      expect(store.lastSave).to.be.greaterThan(1533542388913);
      done();
    });
    store.stopWorker();
  });
  it('should stop a worker', () => {
    const store = new Storage(storageOpts);
    const result = store.stopWorker();
    const expected = true;
    expect(result).to.equal(expected);
    expect(store.interval).to.equal(null);
  });
  it('should clearAll', async () => {
    const store = new Storage(storageOpts);
    await store.clearAll();
    const result = store.getStore();

    const expected = {
      transactions: {},
      wallets: {},
    };
    expect(result).to.deep.equal(expected);
    store.stopWorker();
  });
  it('should fail on import invalid address', () => {
    const store = new Storage(storageOpts);
    const expected = 'Expected path to import an address';
    expect(() => store.importAddresses({ aw: {} }, 'fad183cbf7')).to.throw(expected);
    store.stopWorker();
  });
  it('should fail on import address without walletId', () => {
    const store = new Storage(storageOpts);
    const expected = 'Expected walletId to import addresses';
    expect(() => store.importAddresses({
      yizmJb63ygipuJaRgYtpWCV2erQodmaZt8: miscFixture.yizmJb63ygipuJaRgYtpWCV2erQodmaZt8,
    })).to.throw(expected);
    store.stopWorker();
  });
  it('should fail on import transaction', () => {
    const store = new Storage(storageOpts);
    const expected = 'Can\'t import this transaction. Invalid structure.';
    expect(() => store.importTransactions({ aw: { txid: 'aw' } })).to.throw(expected);
    store.stopWorker();
  });
  it('should fail on update address', () => {
    const store = new Storage(storageOpts);
    const expected = 'Expected path to update an address';
    const expected2 = 'Expected walletId to update an address';
    expect(() => store.updateAddress({ aw: {} }, 'fad183cbf7')).to.throw(expected);
    expect(() => store.updateAddress({ aw: {} })).to.throw(expected2);
    store.stopWorker();
  });
  it('should fail on update tx', () => {
    const store = new Storage(storageOpts);
    const expected = 'Expected a transaction to update';
    expect(() => store.updateTransaction()).to.throw(expected);
    store.stopWorker();
  });
  it('should fail on addNewtxtoAddress', () => {
    const store = new Storage(storageOpts);
    const expected = 'Invalid tx to add : tx';
    expect(() => store.addNewTxToAddress({ aw: {} }), 'fad183cbf7').to.throw(expected);
    store.stopWorker();
  });
  it('should not create a wallet twice', () => {
    const wid = '12345';
    const wid2 = '12346';
    const store = new Storage({ ...storageOpts, walletId: wid });
    expect(store.createWallet(wid)).to.equal(false);
    expect(store.createWallet(wid2)).to.equal(true);
    store.stopWorker();
  });
  it('should', () => {
    console.log();
  });
});
