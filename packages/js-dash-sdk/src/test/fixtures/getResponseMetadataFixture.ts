const Metadata = require('@dashevo/dapi-client/lib/methods/platform/response/Metadata');

function getResponseMetadataFixture() {
  const metadata = {
    height: 10,
    coreChainLockedHeight: 42,
    timeMs: new Date().getTime(),
    protocolVersion: 1,
  };

  return new Metadata(metadata);
}

export default getResponseMetadataFixture;
