const crypto = require('crypto');
const EncodedBuffer = require('../../util/encoding/EncodedBuffer');

/**
 * Generate random identity ID
 *
 * @return {EncodedBuffer}
 */
function generateRandomId() {
  const randomBytes = crypto.randomBytes(36);

  return new EncodedBuffer(
    crypto.createHash('sha256').update(randomBytes).digest(),
    EncodedBuffer.ENCODING.BASE58,
  );
}

module.exports = generateRandomId;
