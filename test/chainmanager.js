const utils = require('../lib/utils');
const config = require('../config/config');


function getNewBlock(prev, bits) {
  return utils.createBlock(prev, parseInt(bits, 16));
}

function generateHeaders() {
  const blocks = [];

  // chain 1 block 1 - connects to genesis
  blocks.push(getNewBlock(config.getLowDiffGenesis(), '1fffffff')); // 0

  // chain 2 block 1 - connects to genesis
  blocks.push(getNewBlock(config.getLowDiffGenesis(), '1fffff0d')); // 1

  // chain 2 block 2
  blocks.push(getNewBlock(blocks[1], '1fffff0c')); // 2

  // chain 1 block 2
  blocks.push(getNewBlock(blocks[0], '1ffffffd')); // 3

  // chain 2 block 3 - first matured block & cumalative difficulty higher than chain 1
  // thus the first block considered main chain
  blocks.push(getNewBlock(blocks[2], '1fffff0b')); // 4

  return blocks;
}


module.exports = {
  fetchHeaders() {
    return generateHeaders();
  },
};
