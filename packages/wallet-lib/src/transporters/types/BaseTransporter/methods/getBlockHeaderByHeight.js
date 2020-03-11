const { is } = require('../../../../utils');

module.exports = async function getBlockHeaderByHeight(height) {
  if (!is.num(height)) throw new Error('Received an invalid height.');
  throw new Error('Not implemented');
};
