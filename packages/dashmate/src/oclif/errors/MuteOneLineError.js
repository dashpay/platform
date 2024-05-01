import { settings } from '@oclif/core';
import AbstractError from '../../errors/AbstractError.js';

export default class MuteOneLineError extends AbstractError {
  /**
   * @param {Error} error
   */
  constructor(error) {
    super('SIGINT');

    if (settings.debug) {
      this.name = error.name;
      this.message = error.message;
      this.stack = error.stack;
    }

    this.error = error;
  }

  /**
   * Get thrown error
   * @return {Error}
   */
  getError() {
    return this.error;
  }
}
