const { is } = require('../../../utils');
const logger = require('../../../logger');

module.exports = async function getAddressSummary(address) {
  if (!is.address(address)) throw new Error('Received an invalid address to fetch');

  logger.silly(`DAPIClient.getAddressSummary[${address}]`);

  const summary = await this.client.core.getAddressSummary(address);

  if (summary.transactions && summary.transactions.length) {
    // With DAPI, the oldest is the last element of the array
    // We do not want to force other transport to also do the same,
    // therefore we reverse it directly here
    summary.transactions.reverse();
  }

  return summary;
};
