/**
 * Allow to clear the working interval (worker).
 * @return {boolean}
 */
module.exports = function stopWorker() {
  clearInterval(this.interval);
  this.interval = null;
  return true;
};
