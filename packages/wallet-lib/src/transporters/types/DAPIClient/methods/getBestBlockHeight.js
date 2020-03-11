module.exports = async function getBestBlockHeight() {
  // Previously we would have done getBlock(hash).height
  return (await this.getStatus()).blocks;
};
