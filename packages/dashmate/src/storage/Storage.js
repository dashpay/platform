const storage = require('node-persist');

const path = require('path');
const {HOME_DIR_PATH} = require("../constants");

class Storage {
  /**
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   */
  constructor(docker, startedContainers) {
    storage.init({dir: path.join(HOME_DIR_PATH)})
  }

  async setItem(key, value) {
    return storage.setItem(key, value)
  }

  async getItem(key, value) {
    const item = await storage.getItem(key,value)

    if(!item) {
      return null
    }

    return item
  }
}

module.exports = Storage;
