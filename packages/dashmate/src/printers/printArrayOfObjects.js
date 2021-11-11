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
  switch (format) {
    case OUTPUT_FORMATS.TABLE: {
      // Init array with headings
      const rows = [Object.keys(array[0])];
      array.map((obj) => rows.push(Object.values(obj)));

      const tableConfig = {
        drawHorizontalLine: (index, size) => index === 0 || index === 1 || index === size,
      };

      output = table(rows, tableConfig);
      break;
    }
    case OUTPUT_FORMATS.JSON: {
      const cleanArray = [];
      array.forEach((outputRow, i) => {
        const cleanRow = {};
        Object.keys(outputRow).forEach((key) => {
          cleanRow[key] = stripAnsi(outputRow[key]);
        });
        cleanArray[i] = cleanRow;
      });
      output = JSON.stringify(cleanArray);
      break;
    }
    default: {
      // eslint-disable-next-line no-console
      console.log('Unsupported format!');
      break;
    }
  }

  // eslint-disable-next-line no-console
  console.log(output);
}

module.exports = printArrayofObjects;
