const { Block } = require('@dashevo/dashcore-lib');
const fs = require('fs');
const blocks = require('../data/blocks/blocks');

module.exports = async function getBlockByHash(hash) {
  const height = blocks.hashes[hash];
  const blockfile = JSON.parse(fs.readFileSync(`${__dirname}/../data/blocks/${height}.json`));
  return new Block(Buffer.from(blockfile.block, 'hex'));
};
