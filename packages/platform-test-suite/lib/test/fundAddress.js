const {
  Transaction,
} = require('@dashevo/dashcore-lib');

const wait = require('../wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {Address} faucetAddress
 * @param {PrivateKey} faucetPrivateKey
 * @param {Address} address
 * @param {number} amount
 * @return {Promise<string>}
 */
async function fundAddress(dapiClient, faucetAddress, faucetPrivateKey, address, amount) {
  const { items: inputs } = await dapiClient.getUTXO(faucetAddress);

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .to(address, amount)
    .change(faucetAddress)
    .fee(668)
    .sign(faucetPrivateKey);

  let { blocks: currentBlockHeight } = await dapiClient.getStatus();

  const transactionId = await dapiClient.sendTransaction(transaction.toBuffer());

  const desiredBlockHeight = currentBlockHeight + 2;

  do {
    ({ blocks: currentBlockHeight } = await dapiClient.getStatus());
    await wait(30000);
  } while (currentBlockHeight < desiredBlockHeight);

  return transactionId;
}

module.exports = fundAddress;
