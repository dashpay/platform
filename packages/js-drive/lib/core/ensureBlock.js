const ZMQClient = require('@dashevo/dashd-zmq');

/**
 *
 * @param {ZMQClient} zmqClient
 * @param {RpcClient} rpcClient
 * @param {string} hash
 * @return {Promise<void>}
 */
async function ensureBlock(zmqClient, rpcClient, hash) {
  const eventPromise = new Promise((resolve) => {
    const onHashBlock = (response) => {
      if (hash === response) {
        zmqClient.removeListener(ZMQClient.TOPICS.hashblock, onHashBlock);

        resolve(response);
      }
    };

    zmqClient.on(ZMQClient.TOPICS.hashblock, onHashBlock);
  });

  try {
    await rpcClient.getBlock(hash);
  } catch (e) {
    // Block not found
    if (e.code === -5) {
      await eventPromise;
    } else {
      throw e;
    }
  }
}

module.exports = ensureBlock;
