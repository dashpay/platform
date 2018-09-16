const {
  dashToDuffs, duffsToDash, getBytesOf, hasProp,
} = require('./utils');
const { generateNewMnemonic, mnemonicToHDPrivateKey, mnemonicToWalletId } = require('./mnemonic.js');
const is = require('./is');
const coinSelection = require('./coinSelection');
const feeCalculation = require('./feeCalculation');
const { hash, hash256, sha256 } = require('./crypto');

module.exports = {
  dashToDuffs,
  duffsToDash,
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
  is,
  coinSelection,
  feeCalculation,
  getBytesOf,
  hash,
  hash256,
  sha256,
  hasProp,
};
