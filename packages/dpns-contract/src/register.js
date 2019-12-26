/* eslint-disable import/no-extraneous-dependencies */
const { client: { http: JaysonClient } } = require('jayson');

const {
  PrivateKey,
  PublicKey,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');
const DAPIClient = require('@dashevo/dapi-client');

const dpnsDocumentsSchema = require('./schema/dpns-documents.json');

/**
 * Execute DPNS contract registration
 *
 * @returns {Promise<DataContract>}
 */
async function register() {
  const seeds = process.env.DAPI_CLIENT_SEEDS
    .split(',')
    .map((ip) => ({ service: `${ip}:${process.env.DAPI_CLIENT_PORT}` }));

  const dapiClient = new DAPIClient({
    seeds,
    timeout: 30000,
  });

  const tendermintRPCClient = new JaysonClient({
    host: process.env.TENDERMINT_RPC_HOST,
    port: process.env.TENDERMINT_RPC_PORT,
  });

  const validationlessDPP = new DashPlatformProtocol({
    dataProvider: {},
  });

  const dpp = new DashPlatformProtocol({
    dataProvider: {
      fetchIdentity: async (id) => {
        const data = Buffer.from(id).toString('hex');

        const {
          result: {
            response: {
              value: serializedIdentity,
            },
          },
        } = await tendermintRPCClient.request(
          'abci_query',
          {
            path: '/identity',
            data,
          },
        );

        if (!serializedIdentity) {
          return null;
        }

        return validationlessDPP.identity.createFromSerialized(
          Buffer.from(serializedIdentity, 'base64'),
          { skipValidation: true },
        );
      },
    },
  });

  const dpnsUserPrivateKey = new PrivateKey(
    process.env.DPNS_USER_PRIVATE_KEY,
  );

  const dpnsUserPublicKey = new PublicKey(
    process.env.DPNS_USER_PUBLIC_KEY,
  );

  const dataContract = dpp.dataContract.create(
    process.env.DPNS_IDENTITY_ID,
    dpnsDocumentsSchema,
  );

  const dataContractST = dpp.dataContract.createStateTransition(dataContract);
  dataContractST.sign(dpnsUserPublicKey, dpnsUserPrivateKey);

  await dapiClient.applyStateTransition(dataContractST);

  return dataContract;
}

module.exports = register;
