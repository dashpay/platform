const { is } = require('../../../../utils');

module.exports = async function getBlockHeaderByHash(hash) {
  if (!is.string(hash)) throw new Error('Received an invalid hash.');
  throw new Error('Not implemented');
};
