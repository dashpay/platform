const logger = require('../../../../logger');

module.exports = async function getBlockHash(hash) {
  logger.silly(`DAPIClient.getBlockHash[${hash}]`);
  return this.client.getBlockHash(hash);
};
