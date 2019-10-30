const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const Reference = require('../../stateView/revisions/Reference');

const getBlocksFixture = require('./getBlocksFixture');
const getStateTransitionsFixture = require('./getStateTransitionsFixture');

/**
 * @param {number} [blockHeight]
 * @return {Reference}
 */
function getReferenceFixture(blockHeight = 1) {
  const blocks = getBlocksFixture();
  const stateTransitions = getStateTransitionsFixture();

  const block = blocks[blockHeight - 1];
  const stateTransition = stateTransitions[blockHeight - 1];

  let hash;
  if (stateTransition.getType() === stateTransitionTypes.DATA_CONTRACT) {
    hash = stateTransition.getDataContract().hash();
  } else if (stateTransition.getType() === stateTransitionTypes.DOCUMENTS) {
    hash = stateTransition.getDocuments()[0].hash();
  }

  return new Reference({
    blockHash: block.hash,
    blockHeight: block.height,
    stHash: stateTransition.hash(),
    hash,
  });
}

module.exports = getReferenceFixture;
