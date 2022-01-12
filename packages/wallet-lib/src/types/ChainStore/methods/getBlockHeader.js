function getBlockHeader(blockHeaderHash) {
  return this.state.blockHeaders.get(blockHeaderHash);
}

module.exports = getBlockHeader;
