/* eslint-disable import/no-extraneous-dependencies */
const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');
const DAPIClient = require('@dashevo/dapi-client');

const dpnsDocumentsSchema = require('./schema/dpns-documents.json');

/**
 * Execute DPNS contract registration
 *
 * @returns {Promise<{ contractId: string, transitionHash: string }>}
 */
async function register() {
  const seeds = process.env.DAPI_CLIENT_SEEDS
    .split(',')
    .map(ip => ({ service: `${ip}:${process.env.DAPI_CLIENT_PORT}` }));

  const dapiClient = new DAPIClient({
    seeds,
    timeout: 30000,
  });

  const dpp = new DashPlatformProtocol();

  const contract = dpp.contract.create('DPNSContract', dpnsDocumentsSchema);

  dpp.setContract(contract);

  const contractPacket = dpp.packet.create(contract);

  const privateKey = new PrivateKey(
    process.env.DPNS_USER_PRIVATE_KEY_STRING,
  );

  const transaction = new Transaction()
    .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

  transaction.extraPayload
    .setRegTxId(process.env.DPNS_USER_REG_TX_ID)
    .setHashPrevSubTx(process.env.DPNS_USER_PREVIOUS_ST)
    .setHashSTPacket(contractPacket.hash())
    .setCreditFee(1000)
    .sign(privateKey);

  const transitionHash = await dapiClient.sendRawTransition(
    transaction.serialize(),
    contractPacket.serialize().toString('hex'),
  );

  return {
    contractId: contract.getId(),
    transitionHash,
  };
}

module.exports = register;
