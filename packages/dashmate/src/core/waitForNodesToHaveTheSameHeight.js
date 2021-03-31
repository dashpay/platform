/**
 * Wait for all nodes to reach the height of the most advanced chain
 * @param {RpcClient[]} rpcClients
 * @param {number} [timeout] - timeout throw error if blocks aren't synced on all nodes
 * @param {number} [waitTime] - wait interval
 * @return {Promise<void>}
 */
async function waitForNodesToHaveTheSameHeight(rpcClients, timeout = 60000, waitTime = 1000) {
  const heights = await Promise.all(
    rpcClients.map(async (rpc) => {
      const promise = rpc.getBlockCount();

      return promise.then(({ result }) => result);
    }),
  );

  const maxHeight = Math.max(...heights);

  const deadline = Date.now() + timeout;

  let isReady = false;

  while (!isReady) {
    const tips = await Promise.all(
      rpcClients.map((rpc) => {
        const promise = rpc.waitForBlockHeight(maxHeight, waitTime);

        return promise.then(({ result }) => result);
      }),
    );

    const allTipsAreSameHeight = tips
      .filter((tip) => tip.height !== maxHeight)
      .length === 0;

    let allTipsAreSameHash = false;

    if (allTipsAreSameHeight) {
      allTipsAreSameHash = tips
        .filter((tip) => tip.hash !== tips[0].hash)
        .length === 0;

      if (!allTipsAreSameHash) {
        throw new Error('Block sync failed, mismatched block hashes');
      }

      // Exit the cycle once reached this point
      isReady = true;
    }

    if (Date.now() > deadline) {
      throw new Error(`Syncing blocks to height ${maxHeight} timed out`);
    }
  }
}

module.exports = waitForNodesToHaveTheSameHeight;
