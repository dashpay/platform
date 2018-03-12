const fs = require('fs');
const path = require('path');

/**
 * @return {Object[]}
 */
module.exports = function getBlockFixtures() {
  const blocksJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/blocks.json'));
  return JSON.parse(blocksJSON);
};
