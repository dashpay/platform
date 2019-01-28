const fs = require('fs');
const path = require('path');

const getStateTransitionsFixture = require('./getStateTransitionsFixture');

/**
 * @return {Object[]}
 */
module.exports = function getBlockFixtures() {
  const blocksJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/blocks.json'));
  const blocks = JSON.parse(blocksJSON.toString());

  const stateTransitions = getStateTransitionsFixture();

  for (const block of blocks) {
    block.tx = stateTransitions.splice(0, block.tx.length).map(h => h.hash);
  }

  return blocks;
};
