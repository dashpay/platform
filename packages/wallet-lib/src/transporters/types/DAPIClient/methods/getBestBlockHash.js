module.exports = async function getBestBlockHash() {
  return this.client.getBestBlockHash();
};
