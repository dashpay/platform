const crypto = require('crypto');
const Match = require('../Match');

/**
 * @param {AbstractStateTransition} stateTransition
 * @param {string} metricTitle
 * @param {Object} metrics
 * @return {Match}
 */
function createStateTransitionMatch(stateTransition, metricTitle, metrics) {
  const stHash = crypto
    .createHash('sha256')
    .update(stateTransition.toBuffer())
    .digest()
    .toString('hex')
    .toUpperCase();

  return new Match({
    txId: stHash,
    txType: stateTransition.getType(),
    abciMethod: 'deliverTx',
  }, (data) => {
    if (!metrics[metricTitle]) {
      // eslint-disable-next-line no-param-reassign
      metrics[metricTitle] = [];
    }

    metrics[metricTitle].push(data);
  });
}

module.exports = createStateTransitionMatch;
