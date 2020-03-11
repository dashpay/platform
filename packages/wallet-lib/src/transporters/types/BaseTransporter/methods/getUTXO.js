const { is } = require('../../../../utils');

module.exports = async function getUTXO(address) {
  if (!is.address(address)) throw new Error('Received an invalid address to fetch');
  throw new Error('Not implemented');
};
