"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createFakeInstantLock = void 0;
var dashcore_lib_1 = require("@dashevo/dashcore-lib");
function createFakeInstantLock(transactionHash) {
    return new dashcore_lib_1.InstantLock({
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
exports.createFakeInstantLock = createFakeInstantLock;
//# sourceMappingURL=createFakeIntantLock.js.map