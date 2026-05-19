import simpleAscendingAccumulator from './simpleAscendingAccumulator.js';
import simpleDescendingAccumulator from './simpleDescendingAccumulator.js';

const STRATEGIES = {
  simpleDescendingAccumulator,
  simpleAscendingAccumulator,
};
export default STRATEGIES;
export { simpleAscendingAccumulator, simpleDescendingAccumulator };