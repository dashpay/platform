const fs = require('fs');
const path = require('path');

const getTransitionHeaderFixtures = require('./getStateTransitionsFixture');

/**
 * @return {Object[]}
 */
module.exports = function getBlockFixtures() {
  const blocksJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/blocks.json'));
  const blocks = JSON.parse(blocksJSON.toString());

  const stHeaders = getTransitionHeaderFixtures();

  for (const block of blocks) {
    block.tx = stHeaders.splice(0, block.tx.length).map(h => h.hash);
  }

  return blocks;
};
