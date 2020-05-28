const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const searchWallet = require('./searchWallet');

describe('Storage - searchWallet', function suite() {
  this.timeout(10000);
  it('should find a wallet', () => {
    const self = {
      store: {
        wallets: {
          '123ae': {
            accounts: {},
            network: Dashcore.Networks.testnet,
            mnemonic: null,
            type: null,
            blockHeight: 0,
            addresses: { external: {}, internal: {}, misc: {} },
          },
        },
        chains: { testnet: { name: 'testnet', blockHeight: -1 } },
      },
    };
    self.getStore = () => self.store;

    const existingWalletid = '123ae';
    const search = searchWallet.call(self, existingWalletid);

    expect(search.found).to.be.equal(true);
    expect(search.walletId).to.be.equal(existingWalletid);
    expect(search.result.accounts).to.be.deep.equal({});
    expect(search.result.addresses).to.be.deep.equal({ external: {}, internal: {}, misc: {} });
  });
});
