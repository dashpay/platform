/**
 * Allow to start the working interval (worker for saving state).
 * @return {void}
 */
export default function startWorker() {
  this.interval = setInterval(() => {
    if (this.lastModified > this.lastSave) {
      this.saveState();
    }
  }, this.autosaveIntervalTime);
};
