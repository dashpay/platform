const stripAnsi = require('strip-ansi');
const { table } = require('table');

const { OUTPUT_FORMATS } = require('../constants');

const UnsupportedFormatError = require('./errors/UnsupportedFormatError');

/**
 * Prints object using specified output format
 *
 * @param {Object} object
 * @param {string} format
 */
function printObject(object, format) {
  let output;
  switch (format) {
    case OUTPUT_FORMATS.PLAIN: {
      const rows = Object.entries(object);
      output = table(rows, { singleLine: true });
      break;
    }
    case OUTPUT_FORMATS.JSON: {
      const cleanObject = {};
      Object.keys(object).forEach((key) => {
        cleanObject[key] = stripAnsi(object[key]);
      });
      output = JSON.stringify(cleanObject);
      break;
    }
    default: {
      throw new UnsupportedFormatError(format);
    }
  }

  // eslint-disable-next-line no-console
  console.log(output);
}

module.exports = printObject;
