const process = require('process');

class ExecutionTimer {
  constructor() {
    this.timers = {};
  }

  /**
   * Start named timer
   *
   * @param {string} name
   *
   * @return {void}
   */
  startTimer(name) {
    if (this.#isStarted(name)) {
      throw new Error(`${name} timer is already started`);
    }

    const timer = process.hrtime();

    this.timers[name] = timer;
  }

  /**
   * End named timer and get timings
   *
   * @param {string} name
   *
   * @return {{ seconds: number, nanoseconds: number }}
   */
  endTimer(name) {
    if (!this.#isStarted(name)) {
      throw new Error(`${name} timer is not started`);
    }

    const timings = process.hrtime(this.timers[name]);

    delete this.timers[name];

    return {
      seconds: timings[0],
      nanoseconds: timings[1],
    };
  }

  #isStarted(name) {
    return this.timers[name] !== undefined;
  }
}

module.exports = ExecutionTimer;
