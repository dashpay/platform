class WalletLibError extends Error {
  constructor(...params) {
    super(...params);

    this.name = this.constructor.name;
  }

  /**
   * @returns {string}
   */
  toString() {
    let string = super.toString();

    if (this.error) {
      string += `\n\n${this.error.toString()}`;
    }

    return string;
  }
}

module.exports = WalletLibError;
