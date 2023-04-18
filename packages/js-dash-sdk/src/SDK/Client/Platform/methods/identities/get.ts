// @ts-ignore
import { Identifier, Metadata } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

/**
 * Get an identity from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string|Identifier} id - id
 * @returns Identity
 */
export async function get(this: Platform, id: Identifier | string): Promise<any> {
  await this.initialize();

  const identifier = Identifier.from(id);

  let identityResponse;
  try {
    identityResponse = await this.client.getDAPIClient().platform
      .getIdentity(identifier);
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const identity = this.dpp.identity.createFromBuffer(identityResponse.getIdentity());

  let metadata;
  const responseMetadata = identityResponse.getMetadata();
  if (responseMetadata) {
    metadata = new Metadata({
      blockHeight: responseMetadata.getHeight(),
      coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
      timeMs: responseMetadata.getTimeMs(),
      protocolVersion: responseMetadata.getProtocolVersion(),
    });
  }

  identity.setMetadata(metadata);

  return identity;
}

export default get;
