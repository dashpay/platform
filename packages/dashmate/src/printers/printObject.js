const { table } = require('table');

const { OUTPUT_FORMATS } = require('../constants');

const UnsupportedFormatError = require('./errors/UnsupportedFormatError');

/**
 * Prints object using specified output format
 *
 * @param {Object} object
 * @param {string} format
 * @param {boolean} flatten
 */
function printObject(object, format, flatten = true) {
  let output;
  switch (format) {
    case OUTPUT_FORMATS.PLAIN: {
      output = table(flatten ? Object.entries(object) : object, { singleLine: true });
      break;
    }
    case OUTPUT_FORMATS.JSON: {
      output = JSON.stringify(object);
      break;
    }
    default: {
      throw new UnsupportedFormatError(format);
    }
  }

  // eslint-disable-next-line no-console
  console.log(output);

  return output;
}

module.exports = printObject;
