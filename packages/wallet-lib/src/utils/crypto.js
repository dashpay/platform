import crypto from 'crypto';

function hash(alg, data) {
  return crypto.createHash(alg).update(data).digest();
}

function sha256(data) {
  return hash('sha256', data);
}

function doubleSha256(data) {
  return sha256(sha256(data));
}

export {
  hash,
  doubleSha256,
  sha256,
};

export default {
  hash,
  doubleSha256,
  sha256,
};
