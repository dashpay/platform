const TYPES = require('./types');

const DocumentsBenchmark = require('./DocumentsBenchmark');
const FunctionBenchmark = require('./FunctionBenchmark');

module.exports = {
  [TYPES.DOCUMENTS]: DocumentsBenchmark,
  [TYPES.FUNCTION]: FunctionBenchmark,
};
