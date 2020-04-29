const { Transaction } = require('@dashevo/dashcore-lib');
const { toSatoshi } = require('../../util/satoshiConverter');

/**
 * Send Dash to address
 *
 * @typedef {sendToAddress}
 * @param {CoreService} coreService
 * @param {string} fundSourcePrivateKey
 * @param {string} fundSourceAddress
 * @param {string} address
 * @param {number} amount Amount in dash
 * @return {Promise<string>}
 */
async function sendToAddress(
  coreService,
  fundSourcePrivateKey,
  fundSourceAddress,
  address,
  amount,
) {
  const fee = 10000;

  const amountToSend = toSatoshi(amount);

  const { result: utxos } = await coreService
    .getRpcClient()
    .getAddressUtxos({ addresses: [fundSourceAddress] });

  const sortedUtxos = utxos
    .sort((a, b) => a.satoshis > b.satoshis);

  const inputs = [];
  let sum = 0;
  let i = 0;

  do {
    const input = sortedUtxos[i];
    inputs.push(input);
    sum += input.satoshis;

    ++i;
  } while (sum < amountToSend + fee && i < sortedUtxos.length);

  const transaction = new Transaction();
  transaction.from(inputs)
    .to(address, amountToSend)
    .change(fundSourceAddress)
    .fee(fee)
    .sign(fundSourcePrivateKey);

  const { result: hash } = await coreService
    .getRpcClient()
    .sendrawtransaction(
      transaction.serialize(),
    );

  return hash;
}

module.exports = sendToAddress;
