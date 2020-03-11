const { is } = require('../../../../utils');

module.exports = async function getBlockByHash(hash) {
  if (!is.string(hash)) throw new Error('Received an invalid hash.');
  throw new Error('Not implemented');
};
