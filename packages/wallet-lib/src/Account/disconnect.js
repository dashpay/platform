/**
 * This method will disconnect from all the opened streams, will stop all running workers
 * and force a saving of the state.
 * You want to use this method at the end of your life cycle of this lib.
 * @return {Boolean}
 */
module.exports = function disconnect() {
  if (this.transport.isValid && this.transport.disconnect) {
    this.transport.disconnect();
  }

  if (this.readinessInterval) {
    this.readinessInterval = clearTimeout(this.readinessInterval);
  }

  if (this.plugins.workers) {
    const workersKey = Object.keys(this.plugins.workers);
    workersKey.forEach((key) => {
      this.plugins.workers[key].stopWorker();
    });
  }
  if (this.storage) {
    this.storage.saveState();
    this.storage.stopWorker();
  }
  if (this.readinessInterval) {
    clearInterval(this.readinessInterval);
  }
  return true;
};
