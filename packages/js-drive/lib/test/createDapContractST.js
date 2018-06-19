const BitcoreLib = require('@dashevo/dashcore-lib');

const { PrivateKey } = BitcoreLib;
const { TransitionPacket, TransitionHeader } = BitcoreLib.StateTransition;

const hashDataMerkleRoot = require('./consensus/hashDataMerkleRoot');

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

  const merkleRoot = await hashDataMerkleRoot(transitionPacket);

  const transitionHeader = new TransitionHeader()
    .setMerkleRoot(merkleRoot)
    .setRegTxHash(userId)
    .sign(privateKey)
    .serialize();

  return [transitionPacket, transitionHeader];
}

module.exports = createDapContractST;
