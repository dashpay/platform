const stripAnsi = require('strip-ansi');
const { table } = require('table');
const { OUTPUT_FORMATS } = require('../constants');

/**
 * Prints object using specified output format
 *
 * @param {[Object[]]} array
 * @param {string} format
 */
function printArrayofObjects(array, format) {
  let output;
  if (format === OUTPUT_FORMATS.TABLE) {
    // Init array with headings
    const rows = [Object.keys(array[0])];

    array.map((obj) => rows.push(Object.values(obj)));

    const tableConfig = {
      drawHorizontalLine: (index, size) => index === 0 || index === 1 || index === size,
    };

    output = table(rows, tableConfig);
  } else if (format === OUTPUT_FORMATS.JSON) {
    array.forEach((outputRow, i) => {
      Object.keys(outputRow).forEach((key) => {
        // eslint-disable-next-line no-param-reassign
        outputRow[key] = stripAnsi(outputRow[key]);
      });
      // eslint-disable-next-line no-param-reassign
      array[i] = outputRow;
    });
    output = JSON.stringify(array);
  }

  // eslint-disable-next-line no-console
  console.log(output);
}

module.exports = printArrayofObjects;
