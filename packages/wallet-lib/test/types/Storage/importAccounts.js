const { expect } = require('chai');
const importAccounts = require('../../../src/types/Storage/methods/importAccounts');
const Wallet = require('../../../src/types/Wallet/Wallet');

describe('Storage - importAccounts', async function suite() {
  this.timeout(15000);
  it('should throw on failed import', () => {
    const mockOpts1 = { };
    const walletId = '123ae';
    const exceptedException1 = 'Expected walletId to import addresses';

    expect(() => importAccounts.call({})).to.throw(exceptedException1);
    expect(() => importAccounts.call({}, walletId)).to.throw(exceptedException1);
  });
  it('should create a wallet if not existing', (done) => {

    const wallet = new Wallet({ offlineMode: true });
    wallet.storage.events.on('CONFIGURED', () => {
      const acc = wallet.getAccount();
      acc.events.on('*', (msg) => { console.log(msg); });
      acc.events.on('INITIALIZED', () => {
        let called = 0;

        const self = {
          searchWallet: () => ({ found: false }),
          createWallet: () => (called += 1),
          store: { wallets: { } },
        };
        self.store.wallets[wallet.walletId] = { accounts: {} };
        importAccounts.call(self, acc, wallet.walletId);

        // Called twice because of recursivity. We have a Acc Instance here.
        expect(called).to.be.equal(2);
        wallet.disconnect();
        acc.disconnect();
        done();
      });
    });
  });
  it('should import an account', (done) => {
    const wallet = new Wallet({ offlineMode: true });
    wallet.storage.events.on('CONFIGURED', () => {
      const acc = wallet.getAccount();
      let called = 0;

      const self = {
        searchWallet: () => ({ found: false }),
        createWallet: () => (called += 1),
        store: { wallets: { } },
      };
      acc.label = 'Heya!';
      self.store.wallets[wallet.walletId] = { accounts: {} };
      importAccounts.call(self, acc, wallet.walletId);
      const walletsKeys = Object.keys(self.store.wallets);
      expect(walletsKeys.length).to.equal(1);
      expect(self.store.wallets[walletsKeys[0]].accounts['m/44\'/1\'/0\''].label).to.equal('Heya!');
      wallet.disconnect();
      acc.disconnect();
      done();
    });
  });
});
