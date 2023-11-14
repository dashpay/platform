import { AbstractError } from '../../errors/AbstractError.js';

export class UnsupportedFormatError extends AbstractError {
  /**
   * @param {string} formatName
   */
  constructor(formatName) {
    super(`Unsupported format: ${formatName}`);

    this.formatName = formatName;
  }

  /**
   * Get config name
   *
   * @return {string}
   */
  getFormatName() {
    return this.formatName;
  }
}
