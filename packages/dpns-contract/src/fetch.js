/* eslint-disable import/no-extraneous-dependencies */
const DAPIClient = require('@dashevo/dapi-client');

/**
 * Fetch DPNS contract
 *
 * @param {string} contractId
 *
 * @returns {Promise<Buffer>}
 */
async function fetch(contractId) {
  const seeds = process.env.DAPI_CLIENT_SEEDS
    .split(',')
    .map((ip) => ({ service: `${ip}:${process.env.DAPI_CLIENT_PORT}` }));

  const dapiClient = new DAPIClient({
    seeds,
    timeout: 30000,
  });

  return dapiClient.getContract(contractId);
}

module.exports = fetch;
