module.exports = async function getBlockHeaderByHeight(blockHeight) {
  return (await this.getBlockByHeight(blockHeight)).header;
};
