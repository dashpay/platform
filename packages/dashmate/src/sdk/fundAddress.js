const {
  Transaction,
} = require('@dashevo/dashcore-lib');

const waitForBlocksWithSDK = require('./waitForBlocksWithSDK');
const generateBlocksWithSDK = require('./generateBlocksWithSDK');
const getInputsByAddress = require('./getInputsByAddress');

const PRESETS = require('../presets');

/**
 *
 * @typedef {fundAddress}
 * @param {DAPIClient} dapiClient
 * @param {string} preset
 * @param {string} network
 * @param {string} faucetAddress
 * @param {PrivateKey} faucetPrivateKey
 * @param {string} address
 * @param {number} amountInSatoshis
 * @return {Promise<string>}
 */
async function fundAddress(
  dapiClient,
  preset,
  network,
  faucetAddress,
  faucetPrivateKey,
  address,
  amountInSatoshis,
) {
  const inputs = await getInputsByAddress(dapiClient, preset, faucetAddress);

  if (!inputs.length) {
    throw new Error(`Address ${faucetAddress} has no inputs to spend`);
  }

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .to(address, amountInSatoshis)
    .change(faucetAddress)
    .sign(faucetPrivateKey);

  const transactionId = await dapiClient.core.broadcastTransaction(transaction.toBuffer());

  if (preset === PRESETS.LOCAL) {
    await generateBlocksWithSDK(dapiClient, network, 1);
  } else {
    await waitForBlocksWithSDK(dapiClient, 1);
  }

  return transactionId;
}

module.exports = fundAddress;
