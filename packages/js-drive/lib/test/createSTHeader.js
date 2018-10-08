const { PrivateKey, Transaction } = require('@dashevo/dashcore-lib');

/**
 * Create DAP contract state transaction packet and header
 *
 * @param {string} regTxId Registration transaction ID (User ID)
 * @param {string} privateKeyString
 * @param {StateTransitionPacket} tsp
 * @param {string} hashPrevSubTx
 * @returns {Promise<Transaction>}
 */
function createSTHeader(regTxId, privateKeyString, tsp, hashPrevSubTx = undefined) {
  const privateKey = new PrivateKey(privateKeyString);

  const extraPayload = {
    version: 1,
    hashSTPacket: tsp.getHash(),
    regTxId,
    creditFee: 1001,
    hashPrevSubTx: (hashPrevSubTx || regTxId),
  };

  const transaction = new Transaction({
    type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
    extraPayload,
  });

  transaction.extraPayload.sign(privateKey);

  return transaction;
}

module.exports = createSTHeader;
