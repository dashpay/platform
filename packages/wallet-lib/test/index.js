const { expect } = require('chai');
const { DAPIClient } = require('@dashevo/dapi-sdk');
const { HDPrivateKey, Script } = require('@dashevo/dashcore-lib');
const {
  createWallet, createTransaction, getNewAddress, sendTransaction, signTransaction,
} = require('../src/index');
const Mnemonic = require('@dashevo/dashcore-mnemonic');

const mnemonic1 = 'knife easily prosper input concert merge prepare autumn pen blood glance toilet';
const dapiClient = new DAPIClient({ port: 3010 });
const mnemonic = new Mnemonic(mnemonic1).toSeed();
const privateHDKey = new HDPrivateKey.fromSeed(mnemonic);

let wallet,
  rawTransaction = null;
xdescribe('Wallet', () => {
  it('should create a wallet from a privateHDKey', () => {
    wallet = createWallet(dapiClient, privateHDKey, 'testnet');

    expect(wallet).to.be.a('object');
    expect(wallet.DAPIClient).to.equal(dapiClient);
    expect(wallet.privateHDKey).to.equal(privateHDKey);
    expect(wallet.synced).to.equal(false);
    expect(wallet).to.have.property('events');
    expect(wallet.network).to.equal('testnet');
  });
  it('should create a wallet from a mnemonic', () => {
    wallet = createWallet(dapiClient, mnemonic1);

    expect(wallet).to.be.a('object');
    expect(wallet.DAPIClient).to.equal(dapiClient);
    expect(wallet.synced).to.equal(false);
    expect(wallet).to.have.property('privateHDKey');
    expect(wallet).to.have.property('events');
    expect(wallet.network).to.equal('livenet');
  });

  it('should generate an address', () => {
    wallet = createWallet(dapiClient, mnemonic1, 'testnet');
    const address = getNewAddress(wallet);
    expect(address).to.equal('yaWEePY8BnKmFGSD6cSjmdiByyV37RsivK');
    expect(getNewAddress(wallet, "m/44'/1'/0'/0/0")).to.equal('yRdxQQpXYh9Xkd91peJ7FJxpEzoRb6droH');
  });
  it('should create a transaction', () => {
    const options = {
      utxos: [{
        address: 'yf6qYQzQoCzpF7gJYAa7s3n5rBK89RoaCQ',
        txId: 'e66474bfe8ae3d91b2784864fc09e0bd615cbfbf4a2164e46b970bcc488a938f',
        outputIndex: 0,
        scriptPubKey: '76a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88ac',
        satoshis: 5000000000,
      }],
      change: 'yf6qYQzQoCzpF7gJYAa7s3n5rBK89RoaCQ',
      to: 'yf6qYQzQoCzpF7gJYAa7s3n5rBK89RoaCQ',
      keys: [privateHDKey.privateKey],
      amount: 10000,
      fee: 9000,
    };

    rawTransaction = createTransaction(wallet, options);

    expect(rawTransaction).to.equal('01000000018f938a48cc0b976be464214abfbf5c61bde009fc644878b2913daee8bf7464e60000000000ffffffff0210270000000000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88acc8a7052a010000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88ac00000000');
  });
  it('should sign a transaction', () => {
    const signedRawTransaction = signTransaction(wallet, rawTransaction);
    // expect(signedRawTransaction).to.equal('01000000018f938a48cc0b976be464214abfbf5c61bde009fc644878b2913daee8bf7464e60000000000ffffffff0210270000000000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88acc8a7052a010000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88ac00000000');
  });
  it('should broadcast a transaction', () => {
    // const rawTx = '01000000018f938a48cc0b976be464214abfbf5c61bde009fc644878b2913daee8bf7464e60000000000ffffffff0210270000000000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88acc8a7052a010000001976a914ce07ed014c455640a41e516ad4cc40fbc7fe435c88ac00000000';
    // expect(sendTransaction(wallet, rawTx)).to.throw(new Error('toto'));
  });
});
