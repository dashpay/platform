/**
 *
 * @param docker
 * @return {stopAllContainers}
 */
function stopAllContainersFactory(docker) {
  /**
   * @typedef {stopAllContainers}
   * @param {string[]} containersIds
   * @param {Object} [options]
   * @param {boolean} [options.remove]
   * @return {Promise<void>}
   */
  async function stopAllContainers(containersIds, options = {}) {
    await Promise.all(containersIds.map(async (containerId) => {
      // stop all containers
      try {
        const container = docker.getContainer(containerId);
        const { State: { Status: status } } = await container.inspect();

        if (status === 'running') {
          await container.stop();

          if (options.remove) {
            await container.remove();
          }
        }
      } catch (e) {
        // just do nothing
      }
    }));
  }

  return stopAllContainers;
}

module.exports = stopAllContainersFactory;
