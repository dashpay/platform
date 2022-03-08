const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const getUTXOS = require('./getUTXOS');
const mockAccountWithStorage = require("../../../test/mocks/mockAccountWithStorage");

describe('Account - getUTXOS', function suite() {
  this.timeout(10000);

  it('should return empty UTXOs list for new account', () => {
    const mockedAccount = mockAccountWithStorage();
    const { walletId, accountPath, network } = mockedAccount;

    // Wipe transactions and addresses from the storage to simulate empty UTXOs
    mockedAccount.storage.getWalletStore(walletId).state.paths.get(accountPath).addresses = {}
    const chainStore = mockedAccount.storage.getChainStore(network);
    chainStore.state.blockHeaders = {};
    chainStore.state.transactions = {};
    chainStore.state.addresses = {};

    const utxos = getUTXOS.call(mockedAccount);

    expect(utxos).to.be.deep.equal([]);
  })

  it('should get the proper UTXOS list', () => {
    const mockedAccount = mockAccountWithStorage();
    const utxos = getUTXOS.call(mockedAccount);

    expect(utxos).to.be.deep.equal([new Dashcore.Transaction.UnspentOutput(
      {
        address: new Dashcore.Address('yMEnFG5TBqEZXYXTg3PhENtZgGbwhw6qbX'),
        txId: '33b14c6bc960c5717d734d5a15dc86b2060bf6e746cc509863344204d356cee4',
        outputIndex: 1,
        script: new Dashcore.Script('76a9140a163cfcba43b87e58b1996f61376d7bd8d9805288ac'),
        satoshis: 224108673,
      },
    )]);
  });
});
