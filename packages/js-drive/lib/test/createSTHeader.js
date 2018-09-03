const { PrivateKey, Transaction } = require('@dashevo/dashcore-lib');

const hashSTPacket = require('./consensus/hashSTPacket');

/**
 * Create DAP contract state transaction packet and header
 *
 * @param {string} regTxId Registration transaction ID (User ID)
 * @param {string} privateKeyString
 * @param {StateTransitionPacket} tsp
 * @returns {Promise<Transaction>}
 */
async function createSTHeader(regTxId, privateKeyString, tsp) {
  const privateKey = new PrivateKey(privateKeyString);

  const stPacketHash = await hashSTPacket(tsp.toJSON({ skipMeta: true }));

  const transaction = new Transaction({
    type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
    extraPayload: {
      version: 1,
      hashSTPacket: stPacketHash,
      regTxId,
      creditFee: 1001,
      hashPrevSubTx: regTxId,
    },
  });

  transaction.extraPayload.sign(privateKey);

  return transaction;
}

module.exports = createSTHeader;
