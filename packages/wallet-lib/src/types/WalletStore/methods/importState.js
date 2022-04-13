const castItemTypes = require('../../../utils/castItemTypes')

const SCHEMA = {
  lastKnownBlock: {
    height: 'number',
  },
};

function importState(state) {
  try {
    castItemTypes(state, SCHEMA)
  } catch (e) {
    console.error(e)
  }

  this.state.lastKnownBlock = state.lastKnownBlock;
}

module.exports = importState;

