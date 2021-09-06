const extendTransactionsWithMetadata = require('./extendTransactionsWithMetadata');
const calculateTransactionFees = require('./calculateTransactionFees');
const categorizeTransactions = require('./categorizeTransactions');
const filterTransactions = require('./filterTransactions');
const { hash, doubleSha256, sha256 } = require('./crypto');
const { varIntSizeBytesFromLength } = require('./varInt');
const classifyAddresses = require('./classifyAddresses');
const feeCalculation = require('./feeCalculation');
const coinSelection = require('./coinSelection');
const fundWallet = require('./fundWallet');
const dashToDuffs = require('./dashToDuffs');
const duffsToDash = require('./duffsToDash');
const getBytesOf = require('./getBytesOf');
const hasMethod = require('./hasMethod');
const hasProp = require('./hasProp');
const is = require('./is');

const {
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
  seedToHDPrivateKey,
  mnemonicToSeed,
} = require('./mnemonic');

module.exports = {
  extendTransactionsWithMetadata,
  varIntSizeBytesFromLength,
  calculateTransactionFees,
  categorizeTransactions,
  mnemonicToHDPrivateKey,
  generateNewMnemonic,
  seedToHDPrivateKey,
  mnemonicToWalletId,
  filterTransactions,
  classifyAddresses,
  mnemonicToSeed,
  feeCalculation,
  coinSelection,
  doubleSha256,
  dashToDuffs,
  duffsToDash,
  fundWallet,
  getBytesOf,
  hasMethod,
  hasProp,
  sha256,
  hash,
  is,
};
