const fs = require('fs').promises;

class FileDb {
  /**
   *
   * @param {string} fileName
   */
  constructor(fileName) {
    this.fileName = fileName;
  }

  /**
   *
   * @return {Promise<Buffer>}
   */
  async get() {
    return fs.readFile(this.fileName);
  }

  /**
   *
   * @param {Buffer} data
   * @return {Promise<void>}
   */
  async set(data) {
    await fs.writeFile(this.fileName, data);
  }

  /**
   *
   * @return {Promise<void>}
   */
  async clear() {
    await fs.truncate(this.fileName);
  }
}

module.exports = FileDb;
