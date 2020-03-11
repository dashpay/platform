module.exports = async function getBlockHeaderByHash(blockHash) {
  return (await this.getBlockByHash(blockHash)).header;
};
