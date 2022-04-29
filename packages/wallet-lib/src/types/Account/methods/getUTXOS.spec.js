const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const getUTXOS = require('./getUTXOS');
const getFixtureHDAccountWithStorage = require("../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage");

describe('Account - getUTXOS', function suite() {
  this.timeout(10000);

  it('should return empty UTXOs list for new account', () => {
    const mockedAccount = getFixtureHDAccountWithStorage();
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
    const mockedAccount = getFixtureHDAccountWithStorage();
    const utxos = getUTXOS.call(mockedAccount);

    const expectedUtxos = [
      {
        "address": "yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x",
        "txid": "c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5",
        "vout": 1,
        "scriptPubKey": "76a914e922f6420544f1be0cb593c10535cc3469198bc888ac",
        "amount": 4
      },
      {
        "address": "yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc",
        "txid": "e6b6f85a18d77974f376f05d6c96d0fdde990e733664248b1a00391565af6841",
        "vout": 1,
        "scriptPubKey": "76a914e9c12479daba9d989cedba69adb56a5a50fe500288ac",
        "amount": 1.59999359
      },
      {
        "address": "yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki",
        "txid": "c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5",
        "vout": 2,
        "scriptPubKey": "76a91404a791e67467246c3c0a003007793160387de54288ac",
        "amount": 1.0709972
      },
      {
        "address": "yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY",
        "txid": "f230a9414bf577d93d6f7f2515d9b549ede78cfba4168920892970fa8aa1eef8",
        "vout": 1,
        "scriptPubKey": "76a91414dfbdcfb48babe7127fa0ee90339c33a46aeda288ac",
        "amount": 0.0009917
      }
    ];

    utxos.forEach((utxo, i) => {
      expect(utxo).to.be.instanceOf(Dashcore.Transaction.UnspentOutput);
      expect(utxo.toObject()).to.be.deep.equal(expectedUtxos[i]);
    })
  });
});
