const { WALLET_TYPES } = require('../../../CONSTANTS');
const logger = require('../../../logger');
/**
 * This will sweep any paper wallet with remaining UTXOS to another Wallet created
 * via a random new mnemonic or via passed one.
 * Will resolves automatically network and transport.
 *
 * By default, the Wallet return is in offlineMode. And therefore sweep will be done
 * on the first address path. You can pass offlineMode:false to overwrite.
 *
 * @param {Wallet.Options} opts - Options to be passed to the wallet swept.
 * @return {Wallet} - Return a new random mnemonic created Wallet.
 */
async function sweepWallet(opts = {}) {
  const self = this;
  // eslint-disable-next-line no-async-promise-executor,consistent-return
  return new Promise(async (resolve, reject) => {
    if (self.walletType !== WALLET_TYPES.SINGLE_ADDRESS) {
      return reject(new Error('Can only sweep wallet initialized from privateKey'));
    }
    const account = await self.getAccount({ index: 0 });
    const currentPublicAddress = account.getAddress().address;
    await account.isReady();
    const balance = await account.getTotalBalance();
    if (!balance > 0) {
      return reject(new Error(`Cannot sweep an empty private key (current balance: ${balance})`));
    }
    let newWallet;
    try {
      const walletOpts = {
        network: self.network,
        transport: self.transport,
        ...opts,
      };
      newWallet = new self.constructor(walletOpts);
      const recipient = newWallet.getAccount({ index: 0 }).getUnusedAddress().address;
      const tx = account.createTransaction({
        satoshis: balance,
        recipient,
      });
      const txid = await account.broadcastTransaction(tx);
      logger.info(`SweepWallet: ${balance} of ${currentPublicAddress} to ${recipient} transfered. Txid :${txid}`);

      return resolve(newWallet);
    } catch (err) {
      await newWallet.disconnect();
      return reject(err);
    }
  });
}
module.exports = sweepWallet;
