import DashCoreLib from '@dashevo/dashcore-lib';
import { toSatoshi } from '../../util/satoshiConverter.js';

const { Transaction } = DashCoreLib;
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
export default async function sendToAddress(
  coreService,
  fundSourcePrivateKey,
  fundSourceAddress,
  address,
  amount,
) {
  const maxFee = 200000;
  const feePerKb = 2000;

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
  } while (sum < amountToSend + maxFee && i < sortedUtxos.length);

  const transaction = new Transaction();
  transaction.from(inputs)
    .to(address, amountToSend)
    .change(fundSourceAddress)
    .feePerKb(feePerKb)
    .sign(fundSourcePrivateKey);

  const { result: hash } = await coreService
    .getRpcClient()
    .sendrawtransaction(
      transaction.serialize(),
    );

  return hash;
}
