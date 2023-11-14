import { table } from 'table';

import { OUTPUT_FORMATS } from '../constants.js';

import UnsupportedFormatError from './errors/UnsupportedFormatError.js';

/**
 * Prints object using specified output format
 *
 * @param {[Object[]]} array
 * @param {string} format
 */
export default function printArrayOfObjects(array, format) {
  let output;
  switch (format) {
    case OUTPUT_FORMATS.PLAIN: {
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
      output = JSON.stringify(array);
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
