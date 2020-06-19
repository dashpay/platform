const wait = require('../wait');

const MAX_TIME_TO_WAIT_MS = 20000;
const ITERATION_TIME_MS = 500;
const NUMBER_OF_ITERATIONS = MAX_TIME_TO_WAIT_MS / ITERATION_TIME_MS;

/**
 * Wait for account balance to change
 *
 * @param {Account} walletAccount
 */
async function waitForBalanceToChange(walletAccount) {
  const originalBalance = walletAccount.getTotalBalance();

  let currentIteration = 0;
  while (walletAccount.getTotalBalance() === originalBalance
      && currentIteration <= NUMBER_OF_ITERATIONS) {
    await wait(ITERATION_TIME_MS);
    currentIteration++;
  }
}

module.exports = waitForBalanceToChange;
