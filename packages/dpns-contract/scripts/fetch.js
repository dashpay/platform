/* eslint-disable import/no-extraneous-dependencies, no-console */
const DAPIClient = require('@dashevo/dapi-client');

const DashPlatformProtocol = require('@dashevo/dpp');

const { argv } = require('yargs')
  .usage(
    'Usage: $0 --dapiAddress [string] --contractId [string]',
  )
  .demandOption(['dapiAddress', 'contractId']);

/**
 * Fetch DPNS contract
 *
 * @returns {Promise<void>}
 */
async function fetch() {
  const seeds = [
    { service: argv.dapiAddress },
  ];

  const dapiClient = new DAPIClient({
    seeds,
    timeout: 30000,
  });

  const dpp = new DashPlatformProtocol({ dataProvider: {} });

  const buffer = await dapiClient.getDataContract(argv.contractId);

  if (!buffer || buffer.length === 0) {
    console.error('Have not been able to found data contract with an id: ', argv.contractId);
    process.exit(1);
  }

  const rawDataContract = await dpp.dataContract.createFromSerialized(
    buffer, { skipValidation: true },
  );

  console.log(
    'Here is the data contract JSON: \n',
    JSON.stringify(rawDataContract, undefined, 2),
  );
}

fetch()
  .catch((e) => console.error(e));
