/**
 * Allow to clear the working interval (worker).
 * @return {boolean}
 */
export default function stopWorker() {
  clearInterval(this.interval);
  this.interval = null;
  return true;
};
