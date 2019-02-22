const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

const StateTransition = require('../../blockchain/StateTransition');

const createPayloadFixture = require('./createPayloadFixture');

/**
 * Create a single StateTransition fixture
 *
 * @param {object} options
 * @param {integer} options.type
 * @param {integer} options.version
 * @param {number} options.fee
 * @param {SubTxTransitionPayload} options.extraPayload
 *
 * @returns {StateTransition}
 */
module.exports = function createStateTransitionFixture({
  type,
  version,
  fee,
  extraPayload,
}) {
  const payload = (extraPayload === undefined ? createPayloadFixture({}) : extraPayload);

  return new StateTransition({
    type: (type === undefined ? Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION : type),
    version: (version === undefined ? 3 : version),
    fee: (fee === undefined ? 0.01 : fee),
    extraPayload: payload.toString(),
  });
};
