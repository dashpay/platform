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
    if (self.walletType !== WALLET_TYPES.PRIVATEKEY) {
      reject(new Error('Can only sweep wallet initialized from privateKey'));
      return;
    }

    const account = await self.getAccount({ index: 0 });
    await account.isReady();

    const balance = await account.getTotalBalance();
    if (balance <= 0) {
      reject(new Error(`Cannot sweep an empty private key (current balance: ${balance})`));
      return;
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

      logger.info(`SweepWallet: ${balance} of ${account.getAddress().address} to ${recipient} transfered. Txid :${txid}`);

      resolve(newWallet);
    } catch (err) {
      if (newWallet) {
        await newWallet.disconnect();
      }

      reject(err);
    }
  });
}
module.exports = sweepWallet;
