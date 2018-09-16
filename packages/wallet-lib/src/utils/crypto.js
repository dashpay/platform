const crypto = require('crypto');

function hash(alg, data) {
  return crypto.createHash(alg).update(data).digest();
}

function sha256(data) {
  return hash('sha256', data);
}

function hash256(data) {
  return sha256(sha256(data));
}

module.exports = {
  hash,
  hash256,
  sha256,
};
