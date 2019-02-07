const _ = require('lodash');
const { is } = require('../utils');
const { InvalidStrategy, UnknownStrategy } = require('../errors');
const buildInStrategies = require('../utils/coinSelections/strategies');

const fromString = function (strategyName) {
  if (!_.has(buildInStrategies, strategyName)) return new UnknownStrategy(`Unknown strategy ${strategyName}`);
  return buildInStrategies[strategyName];
};
const fromFunction = function (arg) {
  return arg;
};

/* eslint-disable no-underscore-dangle */
const _loadStrategy = function (arg) {
  if (is.string(arg)) return fromString(arg);
  if (is.fn(arg)) return fromFunction(arg);
  throw new InvalidStrategy(arg);
};


module.exports = _loadStrategy;
