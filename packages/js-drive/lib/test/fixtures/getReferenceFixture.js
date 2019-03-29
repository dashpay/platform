const Reference = require('../../stateView/revisions/Reference');

const getBlocksFixture = require('./getBlocksFixture');
const getStateTransitionsFixture = require('./getStateTransitionsFixture');
const getSTPacketsFixture = require('./getSTPacketsFixture');

/**
 * @param {number} [blockHeight]
 * @return {Reference}
 */
function getReferenceFixture(blockHeight = 1) {
  const blocks = getBlocksFixture();
  const stateTransitions = getStateTransitionsFixture();
  const stPackets = getSTPacketsFixture();

  const block = blocks[blockHeight - 1];
  const stateTransition = stateTransitions[blockHeight - 1];
  const stPacket = stPackets[blockHeight - 1];

  let hash;
  if (stPacket.getContract()) {
    hash = stPacket.getContract().hash();
  } else {
    hash = stPacket.getDocuments()[0].hash();
  }

  return new Reference({
    blockHash: block.hash,
    blockHeight: block.height,
    stHash: stateTransition.hash,
    stPacketHash: stPacket.hash(),
    hash,
  });
}

module.exports = getReferenceFixture;
