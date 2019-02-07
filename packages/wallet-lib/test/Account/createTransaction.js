const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const _ = require('lodash');
const createTransaction = require('../../src/Account/createTransaction');
const getUnusedAddress = require('../../src/Account/getUnusedAddress');
const getAddress = require('../../src/Account/getAddress');
const generateAddress = require('../../src/Account/generateAddress');
const getPrivateKeys = require('../../src/Account/getPrivateKeys');
const searchTransaction = require('../../src/Storage/searchTransaction');
const getStore = require('../../src/Storage/getStore');
const getUTXOS = require('../../src/Account/getUTXOS');
const KeyChain = require('../../src/KeyChain');
const Storage = require('../../src/Storage/Storage');
const { simpleDescendingAccumulator } = require('../../src/utils/coinSelections/strategies');

const { mnemonicToHDPrivateKey } = require('../../src/utils');

const addressesFixtures = require('../fixtures/addresses.json');
const validStore = require('../fixtures/walletStore').valid.orange.store;
const duringDevelopStore = require('../fixtures/duringdevelop-fullstore-snapshot-1548538361');

const duringDevelopMnemonic = 'during develop before curtain hazard rare job language become verb message travel';

const craftedStrategy = (utxosList, outputsList, deductFee = false, feeCategory = 'normal') => {
  const TransactionEstimator = require('../../src/utils/coinSelections/TransactionEstimator.js');
  const { sortAndVerifyUTXOS } = require('../../src/utils/coinSelections/helpers/');

  const txEstimator = new TransactionEstimator(feeCategory);

  // We add our outputs, theses will change only in case deductfee being true
  txEstimator.addOutputs(outputsList);

  const sort = [{ sortBy: 'satoshis', direction: 'descending' }];
  const sortedUtxosList = sortAndVerifyUTXOS(utxosList, sort);

  const totalOutputValue = txEstimator.getTotalOutputValue();

  let pendingSatoshis = 0;
  const simplyAccumulatedUtxos = sortedUtxosList.filter((utxo) => {
    if (pendingSatoshis < totalOutputValue) {
      pendingSatoshis += utxo.satoshis;
      return utxo;
    }
    return false;
  });
  if (pendingSatoshis < totalOutputValue) {
    throw new Error('Unsufficient utxo amount');
  }

  // We add the expected inputs, which should match the requested amount
  // TODO : handle case when we do not match it.
  txEstimator.addInputs(simplyAccumulatedUtxos);

  const estimatedFee = txEstimator.getFeeEstimate();
  if (deductFee === true) {
    // Then we check that we will be able to do it
    const inValue = txEstimator.getInValue();
    const outValue = txEstimator.getOutValue();
    if (inValue < outValue + estimatedFee) {
      // We don't have enought change for fee, so we remove from outValue
      txEstimator.reduceFeeFromOutput((outValue + estimatedFee) - inValue);
    } else {
      // TODO : Here we can add some process to check up that we clearly have enough to deduct fee
    }
  }
  // console.log('estimatedFee are', estimatedFee, 'satoshis');
  return {
    utxos: txEstimator.getInputs(),
    outputs: txEstimator.getOutputs(),
    feeCategory,
    estimatedFee,
    utxosValue: txEstimator.getInValue(),
  };
};

describe('Account - createTransaction', () => {
  it('sould warn on missing inputs', () => {
    const self = {
      store: validStore,
      walletId: 'a3771aaf93',
      getUTXOS,
    };

    const mockOpts1 = {};
    const mockOpts2 = {
      satoshis: 1000,
    };
    const mockOpts3 = {
      satoshis: 1000,
      recipient: addressesFixtures.testnet.valid.yereyozxENB9jbhqpbg1coE5c39ExqLSaG.addr,
    };
    const expectedException1 = 'An amount in dash or in satoshis is expected to create a transaction';
    const expectedException2 = 'A recipient is expected to create a transaction';
    const expectedException3 = 'Error: utxosList must contain at least 1 utxo';
    expect(() => createTransaction.call(self, mockOpts1)).to.throw(expectedException1);
    expect(() => createTransaction.call(self, mockOpts2)).to.throw(expectedException2);
    expect(() => createTransaction.call(self, mockOpts3)).to.throw(expectedException3);
  });
  it('should create valid transaction', () => {
    const walletId = '5061b8276c';
    const storage = new Storage();
    storage.importAddresses(duringDevelopStore.wallets[walletId].addresses.external, walletId);
    storage.importAddresses(duringDevelopStore.wallets[walletId].addresses.internal, walletId);
    storage.importAccounts(duringDevelopStore.wallets[walletId].accounts, walletId);
    storage.importTransactions(duringDevelopStore.transactions);
    const self = {
      store: duringDevelopStore,
      walletId,
      getUTXOS,
      getUnusedAddress,
      getAddress,
      accountIndex: 0,
      BIP44PATH: 'm/44\'/1\'/0\'',
      getPrivateKeys,
      strategy: simpleDescendingAccumulator ,
      generateAddress,
      keyChain: new KeyChain({ HDRootKey: mnemonicToHDPrivateKey(duringDevelopMnemonic, 'testnet', '') }),
      storage,
      events: { emit: _.noop },
    };

    const txOpts1 = {
      recipient: addressesFixtures.testnet.valid.yereyozxENB9jbhqpbg1coE5c39ExqLSaG.addr,
      satoshis: 1e8,
    };
    const tx1 = createTransaction.call(self, txOpts1);
    expect(tx1.constructor.name).to.equal('Transaction');
    expect(tx1.isFullySigned()).to.equal(true);
    expect(tx1.verify()).to.equal(true);
    const expectedRawTx1 = '0300000001bf4a70ad9d24deb6f374e088208af950c7a2e68d03cfa0a0f3e8e6553d3744dd000000006a47304402202708fb9d0f98720be46cf3db0075e07738a6333475992faf1b8a1e8479c926770220476785d5367c06461af528c975dbd924fcb8cc65c2d412a1bfb377dc675f65340121028614ed50b56e0430d6bb954320b0bc23f0420c3d6e0a2efcd163f414765b6c0cffffffff0200e1f505000000001976a914cb594917ad4e5849688ec63f29a0f7f3badb5da688ac09f41b78030000001976a9149c2e6d97ccb044a3e3ef44319dc1c53cf451988988ac00000000';
    expect(tx1.toString()).to.equal(expectedRawTx1);

    // Only satoshis was changed to equal amount. Should then be similar than first rawtx.
    const txOpts2 = {
      recipient: addressesFixtures.testnet.valid.yereyozxENB9jbhqpbg1coE5c39ExqLSaG.addr,
      amount: 1,
    };
    const tx2 = createTransaction.call(self, txOpts2);
    expect(tx2.toString()).to.equal(expectedRawTx1);

    const tx2Json = tx2.toJSON();
    expect(tx2Json.outputs.length).to.equal(2);
    expect(tx2Json.outputs[0].satoshis).to.equal(1e8);
    expect(tx2Json.outputs[1].satoshis).to.equal(14899999753);
    expect(tx2Json.fee).to.equal(247);

    expect(tx2.outputs[0].script.toAddress().toString()).to.equal('yereyozxENB9jbhqpbg1coE5c39ExqLSaG');
    expect(tx2.outputs[1].script.toAddress().toString()).to.equal('yaZFt1VnAbi72mtyjDNV4AwTECqdg5Bv95');

    expect(tx2.inputs[0].script.toAddress().toString()).to.equal('XsXC8nUKVdN7EQepL5Sg4XDwNvP5zjayY4');
    expect(tx2.inputs[0].output._satoshis).to.equal(15000000000);
  });
  it('should be able to create a transaction with specific strategy', () => {
    const walletId = '5061b8276c';
    const storage = new Storage();
    storage.importAddresses(duringDevelopStore.wallets[walletId].addresses.external, walletId);
    storage.importAddresses(duringDevelopStore.wallets[walletId].addresses.internal, walletId);
    storage.importAccounts(duringDevelopStore.wallets[walletId].accounts, walletId);
    storage.importTransactions(duringDevelopStore.transactions);
    const self = {
      store: duringDevelopStore,
      walletId,
      getUTXOS,
      getUnusedAddress,
      getAddress,
      accountIndex: 0,
      BIP44PATH: 'm/44\'/1\'/0\'',
      getPrivateKeys,
      generateAddress,
      strategy: () => { throw new Error(); }, // Ensure it call the passed option
      keyChain: new KeyChain({ HDRootKey: mnemonicToHDPrivateKey(duringDevelopMnemonic, 'testnet', '') }),
      storage,
      events: { emit: _.noop },
    };
    const txOpts1 = {
      recipient: addressesFixtures.testnet.valid.yereyozxENB9jbhqpbg1coE5c39ExqLSaG.addr,
      satoshis: 100e8,
      strategy: craftedStrategy,
    };
    const tx1 = createTransaction.call(self, txOpts1);
    const expectedRawTx1 = '0300000001bf4a70ad9d24deb6f374e088208af950c7a2e68d03cfa0a0f3e8e6553d3744dd010000006b483045022100da2c81b18703d4a92c644a317e81e9684adad9a1bc200bf45a591d5c7865b3b502206967ee9c23c5292414109e40238561649e284c4af5850d3f8032d3dac184e9df012103fffaacbf96c63a6758b45a5c3ca0d1984781ed6991050169cd578b607d546b4dffffffff0200e40b54020000001976a914cb594917ad4e5849688ec63f29a0f7f3badb5da688ac12b81dd2050000001976a9149c2e6d97ccb044a3e3ef44319dc1c53cf451988988ac00000000';
    expect(tx1.toString()).to.equal(expectedRawTx1);
  });
  it('should be able to have a passed change address', () => {

  });
});
