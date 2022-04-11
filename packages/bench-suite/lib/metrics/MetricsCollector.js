const readline = require('readline');
const events = require('events');
const fs = require('fs');

class MetricsCollector extends events.EventEmitter {
  /**
   * @type {string}
   */
  #driveLogPath;

  /**
   * @type {Match[]}
   */
  #matches = [];

  /**
   * @param {string} driveLogPath
   */
  constructor(driveLogPath) {
    super();

    this.#driveLogPath = driveLogPath;
  }

  /**
   * Add matches
   *
   * @param {Match[]} matches
   */
  addMatches(matches) {
    this.#matches.push(...matches);
  }

  /**
   * @returns {Promise<void>}
   */
  async collect() {
    const rl = readline.createInterface({
      input: fs.createReadStream(this.#driveLogPath),
      crlfDelay: Infinity,
    });

    rl.on('line', (line) => {
      if (line === '') {
        return;
      }

      let logData;
      try {
        logData = JSON.parse(line);
      } catch (e) {
        return;
      }

      this.#applyMatches(logData);
    });

    await events.once(rl, 'close');
  }

  /**
   *
   * @param {Object} data
   */
  #applyMatches(data) {
    this.#matches.forEach((match) => match.applyMatch(data));
  }
}

module.exports = MetricsCollector;
