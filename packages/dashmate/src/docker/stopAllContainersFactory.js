/**
 *
 * @param docker
 * @return {stopAllContainers}
 */
export function stopAllContainersFactory(docker) {
  /**
   * @typedef {stopAllContainers}
   * @param {string[]} containersIds
   * @param {Object} [options]
   * @param {boolean} [options.remove]
   * @return {Promise<void>}
   */
  async function stopAllContainers(containersIds, options = {}) {
    await Promise.all(containersIds.map(async (containerId) => {
      const container = docker.getContainer(containerId);

      try {
        if (options.remove) {
          await container.remove({ force: true });
        } else {
          await container.kill();
        }
      } catch (e) {
        // Skip if container is not found or already stopped
        if (e.statusCode !== 404 && e.statusCode !== 409) {
          throw e;
        }
      }
    }));
  }

  return stopAllContainers;
}
