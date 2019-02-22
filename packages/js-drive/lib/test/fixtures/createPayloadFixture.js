const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');
const Payload = require('@dashevo/dashcore-lib/lib/transaction/payload');

const getSTPacketsFixture = require('./getSTPacketsFixture');

/**
 * Create a single SubTxTransitionPayload fixture
 *
 * @param {object} options
 * @param {string} options.regTxId
 * @param {string} options.hashPrevSubTx
 * @param {string} options.hashSTPacket
 * @param {number} options.creditFee
 * @param {string|PrivateKey} options.privateKey
 *
 * @returns {SubTxTransitionPayload}
 */
module.exports = function createPayloadFixture({
  regTxId,
  hashPrevSubTx,
  hashSTPacket,
  creditFee,
  privateKey,
}) {
  const privateKeyToUse = (privateKey === undefined ? PrivateKey.fromRandom() : privateKey);

  let regTxIdToUse = regTxId;
  if (regTxId === undefined) {
    const payload = new Payload.SubTxRegisterPayload();
    payload.setUserName('drive')
      .setPubKeyIdFromPrivateKey(privateKeyToUse)
      .sign(privateKeyToUse);

    const transaction = new Transaction({
      type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
      version: 3,
      extraPayload: payload.toString(),
    });

    regTxIdToUse = transaction.hash;
  }

  const hashPrevSubTxToUse = (hashPrevSubTx === undefined ? regTxIdToUse : hashPrevSubTx);

  let hashSTPacketToUse = hashSTPacket;
  if (hashSTPacket === undefined) {
    const [stPacket] = getSTPacketsFixture();
    hashSTPacketToUse = stPacket.hash();
  }

  const creditFeeToUse = (creditFee === undefined ? 1001 : creditFee);

  const payload = new Payload.SubTxTransitionPayload();

  payload.setRegTxId(regTxIdToUse)
    .setHashPrevSubTx(hashPrevSubTxToUse)
    .setHashSTPacket(hashSTPacketToUse)
    .setCreditFee(creditFeeToUse)
    .sign(privateKeyToUse);

  return payload;
};
