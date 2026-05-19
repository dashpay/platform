import _ from 'lodash';
import { is } from '../../utils/index.js';
import { InvalidStrategy, UnknownStrategy } from '../../errors/index.js';
import buildInStrategies from '../../utils/coinSelections/strategies/index.js';

const fromString = function fromString(strategyName) {
  if (!_.has(buildInStrategies, strategyName)) return new UnknownStrategy(`Unknown strategy ${strategyName}`);
  return buildInStrategies[strategyName];
};
const fromFunction = function fromFunction(arg) {
  return arg;
};

/* eslint-disable no-underscore-dangle */
const _loadStrategy = function _loadStrategy(arg) {
  if (is.string(arg)) return fromString(arg);
  if (is.fn(arg)) return fromFunction(arg);
  throw new InvalidStrategy(arg);
};

export default _loadStrategy;