module.exports = async function getBlockByHeight(height) {
  const hash = this.blocks.heights[height];
  return this.getBlockByHash(hash);
};
