const { derivePath, getPublicKey } = require('ed25519-hd-key');

const crypto = require('crypto');

/**
 * @typedef generateTenderdashNodeKeyAndId
 * @returns {{
 *   privateKey: Buffer,
 *   publicKey: Buffer,
 *   key: string,
 *   id: string,
 * }}
 */
function generateTenderdashNodeKeyAndId() {
  const derivationPath = "m/9'/5'/3'/4'/0'";

  const seed = crypto.randomBytes(25);

  const { key: privateKey } = derivePath(derivationPath, seed.toString('hex'));

  const publicKey = getPublicKey(privateKey).slice(1);

  const key = Buffer.concat([privateKey, publicKey]);

  const id = crypto.createHash('sha256')
    .update(publicKey)
    .digest('hex')
    .slice(0, 40);

  return {
    privateKey,
    publicKey,
    key: key.toString('base64'),
    id,
  };
}

module.exports = generateTenderdashNodeKeyAndId;
