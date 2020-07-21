const { ValidTransportLayerRequired } = require('../../../errors');

/**
 * @return {Promise<StatusInfo|{blocks:number}>} status
 */
async function fetchStatus() {
  if (!this.transport) throw new ValidTransportLayerRequired('fetchStatus');
  const status = { blocks: -1 };

  try {
    return await this.transport.getStatus();
  } catch (e) {
    status.blocks = await this.transport.getBestBlockHeight();
  }
  return status;
}

module.exports = fetchStatus;
