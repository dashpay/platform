const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

const createPayloadFixture = require('./createPayloadFixture');

/**
 * Create a single StateTransition fixture
 *
 * @param {object} options
 * @param {number} options.type
 * @param {number} options.version
 * @param {number} options.fee
 * @param {SubTxTransitionPayload} options.extraPayload
 *
 * @returns {Transaction}
 */
module.exports = function createStateTransitionFixture({
  type,
  version,
  fee,
  extraPayload,
}) {
  const payload = (extraPayload === undefined ? createPayloadFixture({}) : extraPayload);

  return new Transaction({
    type: (type === undefined ? Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION : type),
    version: (version === undefined ? 3 : version),
    fee: (fee === undefined ? 0.01 : fee),
    extraPayload: payload.toString(),
  });
};
