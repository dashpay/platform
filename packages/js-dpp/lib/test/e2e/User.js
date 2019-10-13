class User {
  /*
   * @param {string} id
   * @param {PrivateKey} privateKey
   *
   * @returns {User}
   */
  constructor(id, privateKey) {
    this.id = id;
    this.privateKey = privateKey;
  }

  /*
   * @returns {string}
   */
  getId() {
    return this.id;
  }

  /*
   * @return {PrivateKey}
   */
  getPrivateKey() {
    return this.privateKey;
  }
}

module.exports = User;
