/**
 * This method will connect to all streams and workers available
 * @return {Boolean}
 */
module.exports = function connect() {
  if (this.transport && this.transport.connect) {
    this.transport.connect();
  }

  if (this.plugins.workers) {
    const workersKey = Object.keys(this.plugins.workers);
    workersKey.forEach((key) => {
      this.plugins.workers[key].startWorker();
    });
  }
  if (this.storage) {
    this.storage.startWorker();
  }
  return true;
};
