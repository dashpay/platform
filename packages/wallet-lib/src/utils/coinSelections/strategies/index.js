const simpleAscendingAccumulator = require('./simpleAscendingAccumulator');
const simpleDescendingAccumulator = require('./simpleDescendingAccumulator');

const STRATEGIES = {
  simpleDescendingAccumulator,
  simpleAscendingAccumulator,
};
module.exports = STRATEGIES;
