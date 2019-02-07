/* eslint-disable max-len */
// Todo : Some validators here are really proto type of methods, urgent impr is needed here.
const { PrivateKey, HDPrivateKey, Transaction } = require('@dashevo/dashcore-lib');

const is = {
  // Primitives
  arr: arr => is.def(arr) && (Array.isArray(arr) || arr.constructor.name === 'Array'),
  num: num => !Number.isNaN(num) && typeof num === 'number',
  float: (float => is.num(float) && Math.floor(float) !== float),
  int: int => Number.isInteger(int) || (is.num(int) && Math.floor(int) === int),
  hex: h => is.string(h) && (h.match(/([0-9]|[a-f])/gim) || []).length === h.length,
  string: str => typeof str === 'string',
  bool: b => b === true || b === false,
  obj: obj => obj === Object(obj),
  fn: fn => typeof fn === 'function',
  type(val, type) { return val && val.constructor.name === type; },
  def: val => val !== undefined,
  undef: val => val === undefined,
  null: val => val === null,
  promise: fn => fn && is.fn(fn.then) && is.fn(fn.catch),
  JSON(val) { try { JSON.stringify(val); return true; } catch (e) { return false; } },
  stringified(val) { try { JSON.parse(val); return true; } catch (e) { return false; } },
  mnemonic: mnemonic => is.string(mnemonic) || is.type(mnemonic, 'Mnemonic'),
  network: network => !is.null(network) && (is.string(network) || is.type(network, 'Network')),
  privateKey: pKey => is.type(pKey, 'PrivateKey') || (is.string(pKey) && PrivateKey.isValid(pKey)),
  HDPrivateKey: hdKey => is.type(hdKey, 'HDPrivateKey') || (is.string(hdKey) && HDPrivateKey.isValidSerialized(hdKey)),
  seed: seed => is.string(seed) || is.privateKey(seed) || is.HDPrivateKey(seed) || is.mnemonic(seed),
  address: addr => is.string(addr) || is.type(addr, 'Address'),
  addressObj: addrObj => is.type(addrObj.address, 'Address') || (is.string(addrObj.address) && is.string(addrObj.path)),
  transactionObj: tx => is.obj(tx) && is.txid(tx.txid) && tx.vin && is.arr(tx.vin) && tx.vout && is.arr(tx.vout),
  feeRate: feeRate => is.obj(feeRate) && is.string(feeRate.type) && is.int(feeRate.value),
  txid: txid => is.string(txid) && txid.length === 64,
  utxo: utxo => is.obj(utxo) && is.txid(utxo.txid) && is.num(utxo.outputIndex) && is.num(utxo.satoshis) && is.string(utxo.scriptPubKey),
  output: output => is.obj(output) && is.num(output.satoshis) && is.address(output.address),
  rawtx: rawtx => is.def(rawtx) && is.hex(rawtx) && (() => { try { Transaction(rawtx); return true; } catch (e) { return false; } })(),
};
// aliases
is.array = is.arr;

module.exports = is;
