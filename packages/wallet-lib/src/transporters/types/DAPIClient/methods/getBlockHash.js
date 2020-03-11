module.exports = async function getBlockHash(height) {
  return this.client.getBlockHash(height);
};
