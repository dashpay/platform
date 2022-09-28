const process = require('process');

class ExecutionTimer {
  /**
   * @type {Object.<string, [number, number]>}
   */
  #started = {};

  /**
   * @type {Object.<string, string>}
   */
  #stopped = {};

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

    this.#started[name] = process.hrtime();
  }

  /**
   * Clear timer
   *
   * @param {string} name
   */
  clearTimer(name) {
    delete this.#started[name];
    delete this.#stopped[name];
  }

  /**
   * Get timer
   *
   * @param {string} name
   * @param {boolean} clear - clear timer after getting
   * @returns {string}
   */
  getTimer(name, clear = false) {
    if (!this.#stopped[name]) {
      throw new Error(`${name} timer is not stopped`);
    }

    const timing = this.#stopped[name];

    if (clear) {
      this.clearTimer(name);
    }

    return timing;
  }

  /**
   * Stop named timer and get timings
   *
   * @param {string} name
   * @param {boolean} keep - do not delete timer
   *
   * @return {string}
   */
  stopTimer(name, keep = false) {
    if (!this.isStarted(name)) {
      throw new Error(`${name} timer is not started`);
    }

    const timings = process.hrtime(this.#started[name]);

    const result = (
      parseFloat(timings[0].toString()) + timings[1] / 1000000000
    ).toFixed(3);

    if (keep) {
      this.#stopped[name] = result;
    }

    return result;
  }

  /**
   * @param {string} name
   * @return {boolean}
   */
  isStarted(name) {
    return this.#started[name] !== undefined;
  }
}

module.exports = ExecutionTimer;
