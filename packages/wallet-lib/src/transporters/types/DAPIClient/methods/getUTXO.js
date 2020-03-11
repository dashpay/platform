const { is } = require('../../../../utils');

module.exports = async function getUTXO(address) {
  if (!is.address(address) && !is.arr(address)) throw new Error('Received an invalid address to fetch');
  return this.client.getUTXO(address);
};
