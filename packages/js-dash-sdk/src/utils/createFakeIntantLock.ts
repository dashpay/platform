import { InstantLock } from "@dashevo/dashcore-lib";

export function createFakeInstantLock(transactionHash: string): InstantLock {
    return new InstantLock({
        txid: transactionHash,
        signature: Buffer.alloc(96).toString('hex'),
        inputs: [
            {
                outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
                outpointIndex: 0,
            },
        ],
    });
}
