const process = require('process');

class ExecutionTimer {
  #timers = {};

  /**
   * Start named timer
   *
   * @param {string} name
   *
   * @return {void}
   */
  startTimer(name) {
    if (this.isStarted(name)) {
      throw new Error(`${name} timer is already started`);
    }

    this.#timers[name] = process.hrtime();
  }

  /**
   * End named timer and get timings
   *
   * @param {string} name
   *
   * @return {number}
   */
  endTimer(name) {
    if (!this.isStarted(name)) {
      throw new Error(`${name} timer is not started`);
    }

    const timings = process.hrtime(this.#timers[name]);

    delete this.#timers[name];

    return (parseFloat(timings[0]) + timings[1] / 1000000000).toFixed(3);
  }

  /**
   * @param {string} name
   * @return {boolean}
   */
  isStarted(name) {
    return this.#timers[name] !== undefined;
  }
}

module.exports = ExecutionTimer;
