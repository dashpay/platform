import { ValidTransportLayerRequired } from '../../../errors/index.js';

/**
 * @return {Promise<Object>} status
 */
async function fetchStatus() {
  if (!this.transport) {
    throw new ValidTransportLayerRequired('fetchStatus');
  }

  return this.transport.getBlockchainStatus();
}

export default fetchStatus;