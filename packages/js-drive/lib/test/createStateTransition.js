const { PrivateKey, Transaction } = require('@dashevo/dashcore-lib');

/**
 * Create DP Contract state transaction packet and ST
 *
 * @param {string} regTxId Registration transaction ID (User ID)
 * @param {string} privateKeyString
 * @param {STPacket} stPacket
 * @param {string} hashPrevSubTx
 * @returns {Promise<Transaction>}
 */
function createStateTransition(regTxId, privateKeyString, stPacket, hashPrevSubTx = undefined) {
  const privateKey = new PrivateKey(privateKeyString);

  const extraPayload = {
    version: 1,
    hashSTPacket: stPacket.hash(),
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

module.exports = createStateTransition;
