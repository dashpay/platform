import { toDash } from '../../util/satoshiConverter.js';

/**
 *
 * @typedef generateToAddress
 * @param {CoreService} coreService
 * @param {number} amount
 * @param {string} address
 * @param {function(balance: number)} [progressCallback]
 * @returns {Promise<void>}
 */

export default async function generateToAddress(
  coreService,
  amount,
  address,
  progressCallback = () => {},
) {
  let addressBalance = 0;

  do {
    await coreService.getRpcClient().generateToAddress(1, address, 10000000);

    const { result: { balance } } = await coreService.getRpcClient().getAddressBalance({
      addresses: [address],
    });

    addressBalance = toDash(balance);

    await progressCallback(addressBalance);
  } while (addressBalance < amount);
}
