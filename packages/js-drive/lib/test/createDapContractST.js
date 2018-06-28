const BitcoreLib = require('@dashevo/dashcore-lib');

const { PrivateKey } = BitcoreLib;
const { TransitionPacket, TransitionHeader } = BitcoreLib.StateTransition;

const hashSTPacket = require('./consensus/hashSTPacket');

/**
 * Create DAP contract state transaction packet and header
 * @param {string} userId
 * @param {string} privateKeyString
 * @param {object} tsp
 * @returns {Promise<[TansitionPacket, TransitionHeader]>}
 */
async function createDapContractST(userId, privateKeyString, tsp) {
  const privateKey = new PrivateKey(privateKeyString);

  const transitionPacket = new TransitionPacket()
    .addObject(tsp);

  const STPacketHash = await hashSTPacket(transitionPacket);

  const transitionHeader = new TransitionHeader()
    .setHashSTPacket(STPacketHash)
    .setRegTxHash(userId)
    .sign(privateKey)
    .serialize();

  return [transitionPacket, transitionHeader];
}

module.exports = createDapContractST;
