const Metadata = require("@dashevo/dapi-client/lib/methods/platform/response/Metadata");

function getResponseMetadataFixture() {
  const metadata = {
    height: 10,
    coreChainLockedHeight: 42,
    signature: Buffer.alloc(12).fill(2),
    getBlockTime: {
      seconds: Math.ceil(new Date().getTime() / 1000),
      nanos: 0,
    },
    // TODO do we want to use Long here?
    protocolVersion: 1,
  };

  return new Metadata(metadata);
}

export default getResponseMetadataFixture;
