const is = require('./is');

function calculateTransactionFees(transaction) {
  if (!is.dashcoreTransaction(transaction)) throw new Error('Expected a valid transaction');
  const { inputs, outputs } = transaction;
  const inputAmount = inputs.reduce((acc, input) => {
    if (!input.output) throw new Error('Expected transaction input to have the output specified');
    return acc + input.output.satoshis;
  }, 0);
  const outputAmount = outputs.reduce((acc, output) => (acc + output.satoshis), 0);
  return transaction.isCoinbase() ? 0 : inputAmount - outputAmount;
}
module.exports = calculateTransactionFees;
