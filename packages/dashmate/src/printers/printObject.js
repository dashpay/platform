import table from 'table';
import { OUTPUT_FORMATS } from '../constants.js';
import UnsupportedFormatError from './errors/UnsupportedFormatError.js';

/**
 * Prints object using specified output format
 *
 * @param {Object} object
 * @param {string} format
 */
export default function printObject(object, format) {
  let output;
  switch (format) {
    case OUTPUT_FORMATS.PLAIN: {
      const rows = Object.entries(object);
      output = table(rows, { singleLine: true });
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
