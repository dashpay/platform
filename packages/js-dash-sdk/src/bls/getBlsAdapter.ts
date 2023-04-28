import loadBLS from '@dashevo/bls';
import { Buffer } from 'buffer';

export default async (): Promise<any> => {
  const bls = await loadBLS();

  return {
    validatePublicKey(publicKeyBuffer: Buffer): boolean {
      let pk;

      try {
        pk = bls.G1Element.fromBytes(Uint8Array.from(publicKeyBuffer));
      } catch (e) {
        return false;
      } finally {
        if (pk) {
          pk.delete();
        }
      }

      return Boolean(pk);
    },
    sign(data: Uint8Array, key: Buffer): Buffer {
      const blsKey = bls.PrivateKey.fromBytes(Uint8Array.from(key), true);
      const signature = bls.BasicSchemeMPL.sign(blsKey, data);
      const result = Buffer.from(signature.serialize());

      signature.delete();
      blsKey.delete();

      return result;
    },
    verifySignature(signature: Buffer, data: Buffer, publicKey: Buffer): boolean {
      const { G1Element, G2Element, BasicSchemeMPL } = bls;

      const blsKey = G1Element.fromBytes(Uint8Array.from(publicKey));

      const blsSignature = G2Element.fromBytes(
        Uint8Array.from(signature),
      );

      const result: boolean = BasicSchemeMPL.verify(blsKey, Uint8Array.from(data), blsSignature);

      blsKey.delete();
      blsSignature.delete();

      return result;
    },
  };
};
