/**
 * Allow to start the working interval (worker for saving state).
 * @return {void}
 */
module.exports = function startWorker() {
  this.interval = setInterval(() => {
    if (this.lastModified > this.lastSave) {
      this.saveState();
    }
  }, this.autosaveIntervalTime);
};
