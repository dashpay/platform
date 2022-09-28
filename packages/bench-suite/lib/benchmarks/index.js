const TYPES = require('./types');

const DocumentsBenchmark = require('./DocumentsBenchmark');
const FunctionBenchmark = require('./FunctionBenchmark');
const StateTransitionsBenchmark = require('./StateTransitionsBenchmark');

module.exports = {
  [TYPES.DOCUMENTS]: DocumentsBenchmark,
  [TYPES.STATE_TRANSITIONS]: StateTransitionsBenchmark,
  [TYPES.FUNCTION]: FunctionBenchmark,
};
