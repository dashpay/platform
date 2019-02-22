const { PrivateKey } = require('@dashevo/dashcore-lib');

const createPayloadFixture = require('./fixtures/createPayloadFixture');
const createStateTransitionFixture = require('./fixtures/createStateTransitionFixture');

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

  const transaction = createStateTransitionFixture({
    extraPayload: createPayloadFixture({
      regTxId,
      hashPrevSubTx,
      hashSTPacket: stPacket.hash(),
      privateKey,
    }),
  });

  return transaction;
}

module.exports = createStateTransition;
