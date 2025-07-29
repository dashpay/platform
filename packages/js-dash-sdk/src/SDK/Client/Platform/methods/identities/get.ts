import { Identifier, Metadata } from '@dashevo/wasm-dpp';
import { GetIdentityResponse } from '@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse';
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

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    const identityId = typeof id === 'string' ? id : id.toString();
    const cacheKey = `identity:${identityId}`;
    
    try {
      // Use cached query for better performance
      const result = await adapter.cachedQuery(cacheKey, async () => {
        return await this.wasmSdk.getIdentity(identityId);
      });
      
      if (!result) {
        return null;
      }

      // Convert wasm-sdk response to js-dash-sdk format if needed
      return adapter.convertResponse(result, 'identity');
    } catch (e) {
      if (e.message?.includes('not found') || e.message?.includes('does not exist')) {
        return null;
      }
      throw e;
    }
  }

  // Legacy implementation - will be removed once migration is complete
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