const { ValidTransportLayerRequired } = require('../../../errors');

async function fetchStatus() {
  if (!this.transport.isValid) throw new ValidTransportLayerRequired('fetchStatus');
  const status = { blocks: -1 };

  const getStatus = await this.transport.getStatus();
  if (getStatus !== false) return getStatus;
  status.blocks = await this.transport.getBestBlockHeight();

  return status;
}
module.exports = fetchStatus;
