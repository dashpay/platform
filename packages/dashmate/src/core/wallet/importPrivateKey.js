/**
 * Import private key into wallet
 *
 * @typedef {importPrivateKey}
 * @param {CoreService} coreService
 * @param {string} privateKey
 * @return {Promise<void>}
 */
async function importPrivateKey(coreService, privateKey) {
  console.log('Importing PK', privateKey);
  return coreService.getRpcClient().importPrivKey(privateKey);
}

module.exports = importPrivateKey;
