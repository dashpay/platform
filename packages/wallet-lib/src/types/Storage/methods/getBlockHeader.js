const { BlockHeaderNotInStore } = require('../../../errors');

/**
 * @param identifier - block hash or height
 * @return {BlockHeader}
 */
const getBlockHeader = function (identifier) {
  const search = this.searchBlockHeader(identifier);
  if (!search.found) throw new BlockHeaderNotInStore(identifier);
  return search.result;
};

module.exports = getBlockHeader;
