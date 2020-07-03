const {
  Transaction,
} = require('@dashevo/dashcore-lib');

const waitForBlocks = require('./waitForBlocks');
const getInputsByAddress = require('./getInputsByAddress');

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

  const transactionId = await dapiClient.sendTransaction(transaction.toBuffer());

  await waitForBlocks(dapiClient, preset, network, 1);

  return transactionId;
}

module.exports = fundAddress;
