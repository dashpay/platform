const pino = require('pino');

const noopLogger = Object.keys(pino.levels.values).reduce((logger, functionName) => ({
  ...logger,
  [functionName]: () => {},
}), {});

module.exports = noopLogger;
