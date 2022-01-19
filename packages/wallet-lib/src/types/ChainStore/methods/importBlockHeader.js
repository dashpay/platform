function importBlockHeader(blockHeader) {
  this.state.blockHeaders.set(blockHeader.hash, blockHeader);
}

module.exports = importBlockHeader;
