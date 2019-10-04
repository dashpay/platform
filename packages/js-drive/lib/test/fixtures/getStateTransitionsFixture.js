const StateTransition = require('../../blockchain/StateTransition');

const createPayloadFixture = require('./createPayloadFixture');
const createStateTransitionFixture = require('./createStateTransitionFixture');

const transitions = [];

/**
 * @param {number} numberOfTransitions
 * @return {StateTransition[]}
 */
function getStateTransitionsFixture(numberOfTransitions = 5) {
  for (let i = transitions.length; i < numberOfTransitions; i++) {
    transitions.push(
      createStateTransitionFixture({
        extraPayload: createPayloadFixture({
          hashPrevSubTx: (i === 0 ? undefined : transitions[i - 1].hash),
        }),
      }),
    );
  }

  return transitions.map(t => new StateTransition(t));
}

module.exports = getStateTransitionsFixture;
