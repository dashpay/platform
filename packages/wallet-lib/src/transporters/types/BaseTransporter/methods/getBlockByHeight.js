const { is } = require('../../../../utils');

module.exports = async function getBlockByHeight(height) {
  if (!is.num(height)) throw new Error('Received an invalid height.');
  throw new Error('Not implemented');
};
