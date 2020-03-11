const { BlockHeaderNotInStore } = require('../../../errors');

const getBlockHeader = function (identifier) {
  const search = this.searchBlockHeader(identifier);
  if (!search.found) throw new BlockHeaderNotInStore(identifier);
  return search.result;
};

module.exports = getBlockHeader;
