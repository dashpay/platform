"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var Metadata = require('@dashevo/dapi-client/lib/methods/platform/response/Metadata');
function getResponseMetadataFixture() {
    var metadata = {
        height: 10,
        coreChainLockedHeight: 42,
        timeMs: new Date().getTime(),
        protocolVersion: 1,
    };
    return new Metadata(metadata);
}
exports.default = getResponseMetadataFixture;
//# sourceMappingURL=getResponseMetadataFixture.js.map