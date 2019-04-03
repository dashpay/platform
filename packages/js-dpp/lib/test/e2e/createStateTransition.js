const { Transaction } = require('@dashevo/dashcore-lib');

/**
 * Create state transition Transaction
 *
 * @param {User} user
 * @param {STPacket} stPacket
 * @param {string} hashPrevSubTx
 * @returns {Transaction}
 */
function createStateTransition(user, stPacket, hashPrevSubTx = undefined) {
  const payload = new Transaction.Payload.SubTxTransitionPayload();

  payload.setRegTxId(user.getId())
    .setHashPrevSubTx(hashPrevSubTx || user.getId())
    .setHashSTPacket(stPacket.hash())
    .setCreditFee(1001)
    .sign(user.getPrivateKey());

  return new Transaction({
    type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
    version: 3,
    fee: 0.01,
    extraPayload: payload.toString(),
  });
}

module.exports = createStateTransition;
