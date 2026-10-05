import crypto from 'crypto';

// Tor's HashedControlPassword format: "16:" + hex(salt) + "60" + hex(hash), where
// the hash is the RFC 2440 iterated-and-salted S2K of salt||password with the
// count indicator 0x60. Same output as `tor --hash-password`.
const S2K_INDICATOR = 0x60;
// RFC 2440 decodes the indicator as (16 + low nibble) * 2^(high nibble + 6);
// for 0x60 that is 16 * 2^12 bytes hashed.
const S2K_COUNT = (16 + (S2K_INDICATOR % 16)) * (2 ** (Math.floor(S2K_INDICATOR / 16) + 6));
const SALT_LENGTH = 8;

/**
 * @param {string} password
 * @param {Buffer} [salt] - for deterministic output in tests
 * @return {string}
 */
export default function hashTorControlPassword(password, salt = crypto.randomBytes(SALT_LENGTH)) {
  if (salt.length !== SALT_LENGTH) {
    throw new Error(`Tor S2K salt must be ${SALT_LENGTH} bytes`);
  }

  const secret = Buffer.concat([salt, Buffer.from(password, 'utf8')]);

  const hash = crypto.createHash('sha1');
  let remaining = S2K_COUNT;
  while (remaining > 0) {
    const chunk = remaining >= secret.length ? secret : secret.subarray(0, remaining);
    hash.update(chunk);
    remaining -= chunk.length;
  }

  return `16:${salt.toString('hex')}${S2K_INDICATOR.toString(16)}${hash.digest('hex')}`.toUpperCase();
}
