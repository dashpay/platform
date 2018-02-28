const fs = require('fs');
const path = require('path');

let blocks;

/**
 * @return {Object[]}
 */
module.exports = function getBlockFixtures() {
  if (!blocks) {
    const blocksJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/blocks.json'));
    blocks = JSON.parse(blocksJSON);
  }

  return blocks;
};
