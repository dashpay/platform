const CryptoJS = require('crypto-js');
const { AES } = CryptoJS;

/**
 * @param {string} method
 * @param {string} data
 * @param {string} secret
 * @param {'hex'|string} [encoding=CryptoJS.enc.Utf8]
 * @return {string}
 */
const decrypt = function decrypt(method, data, secret, encoding = '') {
  let decrypted;
  switch (method) {
    default:
      decrypted = AES.decrypt(data, secret);
      return (encoding === 'hex') ? decrypted.toString(CryptoJS.enc.Hex) : decrypted.toString(CryptoJS.enc.Utf8);
  }
};
module.exports = decrypt;
