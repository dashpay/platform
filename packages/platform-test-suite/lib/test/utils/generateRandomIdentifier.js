import crypto from 'crypto';
import Dash from 'dash';

const { Platform } = Dash;

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
async function generateRandomIdentifier() {
  const { Identifier } = await Platform.initializeDppModule();
  return new Identifier(crypto.randomBytes(32));
}

export default generateRandomIdentifier;
