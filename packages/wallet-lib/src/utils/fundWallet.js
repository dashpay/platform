const EVENTS = require('../EVENTS');

/**
 *
 * @param {Account} walletAccount
 * @param {string} id - transaction id
 * @return {Promise<string>}
 */
function waitForTransaction(walletAccount, id) {
  return new Promise(((resolve) => {
    const listener = (event) => {
      const { payload: { transaction } } = event;

      if (transaction.id === id) {
        walletAccount.removeListener(EVENTS.FETCHED_CONFIRMED_TRANSACTION, listener);

        resolve(transaction.id);
      }
    };

    walletAccount.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, listener);
  }));
}

/**
 *
 * @param {Wallet} faucetWallet
 * @param {Wallet} recipientWallet
 * @param {number} amount
 * @return {Promise<void>}
 */
async function fundWallet(faucetWallet, recipientWallet, amount) {
  const faucetAccount = await faucetWallet.getAccount();
  const recipientAccount = await recipientWallet.getAccount();

  const transaction = await faucetAccount.createTransaction({
    satoshis: amount,
    recipient: recipientAccount.getAddress().address,
  });

  await faucetAccount.broadcastTransaction(transaction);

  await waitForTransaction(recipientAccount, transaction.id);

  return transaction.id;
}

module.exports = fundWallet;
