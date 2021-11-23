import { InstantLock } from "@dashevo/dashcore-lib";

export function createFakeInstantLock(transactionHash: string): InstantLock {
    return new InstantLock({
        version: 1,
        txid: transactionHash,
        signature: Buffer.alloc(96).toString('hex'),
        cyclehash: '0dc8d0df62b076a7757ab5ca07dde0f1e2bfaf83f94299fd9a77577e6cc7022e',
        inputs: [
            {
                outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
                outpointIndex: 0,
            },
        ],
    });
}
