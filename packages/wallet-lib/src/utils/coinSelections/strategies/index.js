const simpleAscendingAccumulator = require('./simpleAscendingAccumulator');
const simpleDescendingAccumulator = require('./simpleDescendingAccumulator');
const simpleTransactionOptimizedAccumulator = require('./simpleTransactionOptimizedAccumulator');

const STRATEGIES = {
  simpleDescendingAccumulator,
  simpleAscendingAccumulator,
  simpleTransactionOptimizedAccumulator,
};
module.exports = STRATEGIES;
