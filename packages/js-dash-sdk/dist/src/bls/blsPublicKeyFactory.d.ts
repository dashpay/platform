export = blsPublicKeyFactory;
/**
 * Create an instance of BlsPrivateKey
 *
 * @param {string|Buffer|Uint8Array|PrivateKey} publicKey string must be hex
 * @returns {Promise<PublicKey>}
 */
declare function blsPublicKeyFactory(
  publicKey: string | Buffer | Uint8Array | any
): Promise<typeof import("@dashevo/bls").G1Element>;
