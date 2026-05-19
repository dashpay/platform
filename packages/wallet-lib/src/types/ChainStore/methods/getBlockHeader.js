function getBlockHeader(blockHeaderHash) {
  return this.state.blockHeaders.get(blockHeaderHash);
}

export default getBlockHeader;