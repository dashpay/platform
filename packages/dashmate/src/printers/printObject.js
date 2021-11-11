const stripAnsi = require('strip-ansi');
const { table } = require('table');
const { OUTPUT_FORMATS } = require('../constants');

/**
 * Prints object using specified output format
 *
 * @param {Object} object
 * @param {string} format
 */
function printObject(object, format) {
  let output;
  if (format === OUTPUT_FORMATS.TABLE) {
    const rows = [];
    for (const [key, value] of Object.entries(object)) {
      rows.push([key, value]);
    }
    output = table(rows, { singleLine: true });
  } else if (format === OUTPUT_FORMATS.JSON) {
    Object.keys(object).forEach((key) => {
      // eslint-disable-next-line no-param-reassign
      object[key] = stripAnsi(object[key]);
    });
    output = JSON.stringify(object);
  }

  // eslint-disable-next-line no-console
  console.log(output);
}

module.exports = printObject;
