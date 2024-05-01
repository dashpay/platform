/**
 * Import private key into wallet
 *
 * @typedef {importPrivateKey}
 * @param {CoreService} coreService
 * @param {string} privateKey
 * @return {Promise<void>}
 */
export default async function importPrivateKey(coreService, privateKey) {
  return coreService.getRpcClient().importPrivKey(privateKey, { wallet: 'main' });
}
