const { ValidTransportLayerRequired } = require('../../../errors');

/**
 * @return {Promise<Object>} status
 */
async function fetchStatus() {
  if (!this.transport) {
    throw new ValidTransportLayerRequired('fetchStatus');
  }

  return this.transport.getStatus();
}

module.exports = fetchStatus;
