const { each } = require('lodash');
const classifyAddresses = require('./classifyAddresses');
const { TRANSACTION_HISTORY_TYPES } = require('../CONSTANTS');

// TODO: On a private key based wallet, as change and external is similar,
//  we actually cannot differentiate correctly from an address_transfer
//  and a sent transaction where our own address is a change...
const determineType = (inputsDetection, outputsDetection) => {
  let type = TRANSACTION_HISTORY_TYPES.UNKNOWN;

  // We first discriminate with account transfer from or to another account
  if (inputsDetection.hasOtherAccountAddress
    && !inputsDetection.hasOwnAddress) {
    type = TRANSACTION_HISTORY_TYPES.ACCOUNT_TRANSFER;
  } else if (inputsDetection.hasOwnAddress
    && outputsDetection.hasOtherAccountAddress
  ) {
    type = TRANSACTION_HISTORY_TYPES.ACCOUNT_TRANSFER;
  } else if (inputsDetection.hasOwnAddress
      && !outputsDetection.hasUnknownAddress
      && !outputsDetection.hasOtherAccountAddress) {
    // Detecting an address transfer is the second element we need to discriminate
    type = TRANSACTION_HISTORY_TYPES.ADDRESS_TRANSFER;
  } else {
    if (inputsDetection.hasExternalAddress) {
      type = TRANSACTION_HISTORY_TYPES.RECEIVED;
    }
    if (outputsDetection.hasExternalAddress && !inputsDetection.hasExternalAddress) {
      type = TRANSACTION_HISTORY_TYPES.RECEIVED;
    }
    if (
      outputsDetection.hasUnknownAddress
        && (inputsDetection.hasOwnAddress)
    ) {
      type = TRANSACTION_HISTORY_TYPES.SENT;
    }
  }

  return type;
};

function categorizeTransactions(
  transactionsWithMetadata,
  walletStore,
  accountIndex,
  walletType,
  network = 'testnet',
) {
  const categorizedTransactions = [];

  const {
    externalAddressesList,
    internalAddressesList,
    otherAccountAddressesList,
  } = classifyAddresses(walletStore, accountIndex, walletType, network);

  each(transactionsWithMetadata, (transactionWithMetadata) => {
    const [transaction, metadata] = transactionWithMetadata;
    const from = [];
    const to = [];

    let outputsHasChangeAddress = false;
    let outputsHasExternalAddress = false;
    let outputsHasOtherAccountAddress = false;
    let outputsHasOwnAddress = false;
    let outputsHasUnknownAddress = false;

    let inputsHasChangeAddress = false;
    let inputsHasExternalAddress = false;
    let inputsHasOtherAccountAddress = false;
    let inputsHasOwnAddress = false;
    let inputsHasUnknownAddress = false;

    // For each vout, we will look at matching known addresses
    transaction.outputs.forEach((vout) => {
      const { satoshis, script } = vout;
      const address = script.toAddress(network).toString();
      if (address) {
        if (internalAddressesList.includes(address)) {
          outputsHasChangeAddress = true;
          outputsHasOwnAddress = true;
        } else if (externalAddressesList.includes(address)) {
          outputsHasExternalAddress = true;
          outputsHasOwnAddress = true;
        } else if (otherAccountAddressesList.includes(address)) {
          outputsHasOtherAccountAddress = true;
        } else outputsHasUnknownAddress = true;
        to.push({
          address,
          satoshis,
        });
      }
    });
    // For each vin, we will look at matching known addresses
    // In order to know the value in, we would require fetching tx for output of vin info
    transaction.inputs.forEach((vin) => {
      const { script } = vin;
      const address = script.toAddress(network).toString();
      if (address) {
        if (internalAddressesList.includes(address)) {
          inputsHasChangeAddress = true;
          inputsHasOwnAddress = true;
        } else if (externalAddressesList.includes(address)) {
          inputsHasExternalAddress = true;
          inputsHasOwnAddress = true;
        } else if (otherAccountAddressesList.includes(address)) {
          inputsHasOtherAccountAddress = true;
        } else inputsHasUnknownAddress = true;
        from.push({
          address,
        });
      }
    });

    const type = determineType({
      hasChangeAddress: inputsHasChangeAddress,
      hasExternalAddress: inputsHasExternalAddress,
      hasOtherAccountAddress: inputsHasOtherAccountAddress,
      hasOwnAddress: inputsHasOwnAddress,
      hasUnknownAddress: inputsHasUnknownAddress,
    }, {
      hasChangeAddress: outputsHasChangeAddress,
      hasExternalAddress: outputsHasExternalAddress,
      hasOtherAccountAddress: outputsHasOtherAccountAddress,
      hasOwnAddress: outputsHasOwnAddress,
      hasUnknownAddress: outputsHasUnknownAddress,
    });

    const categorizedTransaction = {
      from,
      to,
      transaction,
      type,
      blockHash: metadata.blockHash,
      height: metadata.height,
      isInstantLocked: metadata.isInstantLocked,
      isChainLocked: metadata.isChainLocked,
    };
    categorizedTransactions.push(categorizedTransaction);
  });

  return categorizedTransactions;
}

module.exports = categorizeTransactions;
