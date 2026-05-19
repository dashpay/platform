import wasmDpp from '@dashevo/wasm-dpp';
const { Identifier, Metadata } = wasmDpp;
import type { Identifier as IdentifierType } from '@dashevo/wasm-dpp';
type Identifier = IdentifierType;
import GetIdentityResponse from '@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse.js';
import NotFoundError from '@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError.js';
import { Platform } from '../../Platform.js';

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

  let identityResponse: GetIdentityResponse;
  try {
    identityResponse = await this.fetcher.fetchIdentity(identifier);
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }
  const identity = this.dpp.identity.createFromBuffer(identityResponse.getIdentity() as Uint8Array);

  let metadata;
  const responseMetadata = identityResponse.getMetadata();
  if (responseMetadata) {
    metadata = new Metadata(
      responseMetadata.getHeight(),
      responseMetadata.getCoreChainLockedHeight(),
      responseMetadata.getTimeMs(),
      responseMetadata.getProtocolVersion(),
    );
  }

  identity.setMetadata(metadata);

  return identity;
}

export default get;
