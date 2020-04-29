/**
 *
 * @param docker
 * @return {stopAllContainers}
 */
function stopAllContainersFactory(docker) {
  /**
   * @typedef {stopAllContainers}
   * @param {string[]} containersIds
   * @return {Promise<void>}
   */
  async function stopAllContainers(containersIds) {
    await Promise.all(containersIds.map(async (containerId) => {
      // stop all containers
      try {
        const container = docker.getContainer(containerId);
        const { State: { Status: status } } = await container.inspect();

        if (status === 'running') {
          await container.stop();
          await container.remove();
        }
      } catch (e) {
        // just do nothing
      }
    }));
  }

  return stopAllContainers;
}

module.exports = stopAllContainersFactory;
