const { is } = require('../../../../utils');
const logger = require('../../../../logger');

module.exports = async function getAddressSummary(address) {
  if (!is.address(address)) throw new Error('Received an invalid address to fetch');
  logger.silly(`DAPIClient.getAddressSummary[${address}]`);
  return this.client.getAddressSummary(address);
};
