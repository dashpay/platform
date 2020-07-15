const {
  Transaction,
} = require('@dashevo/dashcore-lib');

const waitForBlocks = require('../waitForBlocks');
const getInputsByAddress = require('./getInputsByAddress');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {string} faucetAddress
 * @param {PrivateKey} faucetPrivateKey
 * @param {string} address
 * @param {number} amountInSatoshis
 * @return {Promise<string>}
 */
async function fundAddress(
  dapiClient,
  faucetAddress,
  faucetPrivateKey,
  address,
  amountInSatoshis,
) {
  const inputs = await getInputsByAddress(dapiClient, faucetAddress);

  if (!inputs.length) {
    throw new Error(`Address ${faucetAddress} has no inputs to spend`);
  }

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .to(address, amountInSatoshis)
    .change(faucetAddress)
    .fee(668)
    .sign(faucetPrivateKey);

  const transactionId = await dapiClient.core.broadcastTransaction(transaction.toBuffer());

  await waitForBlocks(dapiClient, 1);

  return transactionId;
}

module.exports = fundAddress;
