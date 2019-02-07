/* eslint-disable no-underscore-dangle */
const _ = require('lodash');
const Dashcore = require('@dashevo/dashcore-lib');
const { CreateTransactionError } = require('../errors');
const { dashToDuffs, coinSelection, is } = require('../utils');
const _loadStrategy = require('./_loadStrategy');

/**
 * Create a transaction based around on the provided information
 * @param opts - Options object
 * @param opts.amount - Amount in dash that you want to send
 * @param opts.satoshis - Amount in satoshis
 * @param opts.recipient - Address of the recipient
 * @param opts.recipients - Optional - replace individual satoshis/amount/recipient args
 * @param opts.change - String - A valid Dash address - optional
 * @param opts.utxos - Array - A utxo set - optional
 * @param opts.isInstantSend - If you want to use IS or stdTx.
 * @param opts.deductFee - Deduct fee
 * @param opts.privateKeys - Overwrite default behavior : auto-searching local matching keys.
 * @param opts.strategy - Overwrite default strategy
 * @return {String} - rawTx
 */
function createTransaction(opts) {
  const self = this;
  const tx = new Dashcore.Transaction();

  let outputs = [];

  if (_.has(opts, 'recipients')) {
    if (!is.arr(opts.recipients)) throw new Error('Expected recipients to be an array of recipient');
    _.each(opts.recipients, (recipient) => {
      if (_.has(recipient, 'address') && _.has(recipient, 'satoshis')) {
        outputs.push(recipient);
      } else {
        console.error('Invalid recipient provided', recipient);
      }
    });
  } else {
    if (!opts || (!opts.amount && !opts.satoshis)) {
      throw new Error('An amount in dash or in satoshis is expected to create a transaction');
    }
    const satoshis = (opts.amount && !opts.satoshis) ? dashToDuffs(opts.amount) : opts.satoshis;
    if (!opts || (!opts.recipient)) {
      throw new Error('A recipient is expected to create a transaction');
    }
    outputs = [{ address: opts.recipient, satoshis }];
  }


  const deductFee = _.has(opts, 'deductFee')
    ? opts.deductFee
    : true;

  const strategy = _.has(opts, 'strategy')
    ? _loadStrategy(opts.strategy)
    : this.strategy;


  const utxosList = _.has(opts, 'utxos') ? opts.utxos : this.getUTXOS();

  utxosList.map((utxo) => {
    const utxoTx = self.storage.searchTransaction(utxo.txid);
    if (utxoTx.found) {
      // eslint-disable-next-line no-param-reassign
      // console.log(utxoTx.result.vin);
      // utxo.scriptSig = utxoTx.result.vin[0].scriptSig.hex;
    }
    return utxo;
  });

  const feeCategory = (opts.isInstantSend) ? 'instant' : 'normal';
  let selection;
  try {
    selection = coinSelection(utxosList, outputs, deductFee, feeCategory, strategy);
  } catch (e) {
    throw new CreateTransactionError(e);
  }

  const selectedUTXOs = selection.utxos;
  const selectedOutputs = selection.outputs;
  const {
    // feeCategory,
    estimatedFee,
  } = selection;

  tx.to(selectedOutputs);


  // We parse our inputs, transform them into a Dashcore UTXO object.
  const inputs = selectedUTXOs.reduce((accumulator, current) => {
    const unspentoutput = new Dashcore.Transaction.UnspentOutput(current);
    accumulator.push(unspentoutput);

    return accumulator;
  }, []);

  if (!inputs) return tx;
  // We can now add direction our inputs to the Dashcore TX object
  tx.from(inputs);

  // In case or excessive fund, we will get that to an address in our possession
  // and determine the finalFees
  // eslint-disable-next-line no-underscore-dangle
  const preChangeSize = tx._estimateSize();
  const changeAddress = _.has(opts, 'change') ? opts.change : this.getUnusedAddress('internal').address;
  tx.change(changeAddress);
  // eslint-disable-next-line no-underscore-dangle
  const deltaChangeSize = tx._estimateSize() - preChangeSize;
  const finalFees = Math.ceil(estimatedFee + (deltaChangeSize * estimatedFee / preChangeSize));

  tx.fee(finalFees);
  const addressList = selectedUTXOs.map((el) => {
    if (el.address) return el.address;
    return Dashcore.Script
      .fromHex(el.scriptPubKey)
      .toAddress(this.network)
      .toString();
  });

  const privateKeys = _.has(opts, 'privateKeys')
    ? opts.privateKeys
    : this.getPrivateKeys(addressList);
  const transformedPrivateKeys = [];
  privateKeys.forEach((pk) => {
    if (pk.constructor.name === 'PrivateKey') {
      transformedPrivateKeys.push(pk);
    } else if (pk.constructor.name === 'HDPrivateKey') transformedPrivateKeys.push(pk.privateKey);
    else {
      console.log('Unexpected pk type', pk, pk.constructor.name);
    }
  });
  try {
    const signedTx = this.keyChain.sign(
      tx,
      transformedPrivateKeys,
      Dashcore.crypto.Signature.SIGHASH_ALL,
    );
    return signedTx;
  } catch (e) {
    // if (e.message === 'Not fully signed transaction') {}
    return e;
  }
}
module.exports = createTransaction;
