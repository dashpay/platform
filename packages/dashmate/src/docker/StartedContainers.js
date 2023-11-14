/**
 *  Store all started docker container IDs
 */

export default class StartedContainers {
  constructor() {
    this.containers = new Set();
  }

  /**
   * Add started docker container ID
   *
   * @param {string} containerId
   */
  addContainer(containerId) {
    this.containers.add(containerId);
  }

  /**
   * Get all started docker container IDs
   *
   * @return {string[]}
   */
  getContainers() {
    return [...this.containers];
  }
}
