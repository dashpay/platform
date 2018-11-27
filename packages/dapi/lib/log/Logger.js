const fs = require('fs');
const { EOL } = require('os');
const util = require('util');

class Logger {
  constructor(options = { level: 'INFO' }) {
    this.outputFilePath = options.outputFilePath;
    this.LEVELS = Object.freeze([
      'FATAL',
      'ERROR',
      'WARN',
      'NOTICE',
      'INFO',
      'DEBUG',
      'VERBOSE',
    ]);
    this.level = (options.level && this.LEVELS.indexOf(options.level.toUpperCase())) || 4;
    if (this.level < 0) {
      throw new Error(`Logger: No log level matches ${options.level}`);
    }

    // Create function for each of the different type of levels
    this.LEVELS.forEach((name, index) => {
      this[name] = index;
      this[name.toLowerCase()] = (...restArgs) => {
        const args = Array.prototype.slice.call(restArgs);// We take all args passed by
        args.unshift(name); // We add the level as first args
        this.log(...args); // And we convert again to arguments
      };
    });
  }

  log(...restArgs) {
    let log = '';
    let level = 4;// By default we display from info to fatal.
    const args = Array.prototype.slice.call(restArgs);

    // We need to check if the first args is one of the level designed.
    if (args && args.length > 1 && this.LEVELS.includes(args[0].toUpperCase())) {
      level = this.LEVELS.indexOf(args[0].toUpperCase());
      args.shift();// Remove the level in order to avoid displaying it.
    }
    args.forEach((el) => {
      if (typeof el === 'string') {
        log += ` ${el}`;
      } else {
        log += ` ${util.inspect(el, false, null)}`;
      }
    });
    if (level <= this.level) {
      if (this.outputFilePath) {
        const appendFileAsync = util.promisify(fs.appendFile);
        try {
          appendFileAsync(this.outputFilePath, EOL + log.trim(), { encoding: 'utf8' });
        } catch (error) {
          // eslint-disable-next-line no-console
          console.error(`Error: Logger: ${error}`);
        }
      } else {
        // eslint-disable-next-line no-console
        console.log(log);
      }
    }
  }
}

module.exports = Logger;
