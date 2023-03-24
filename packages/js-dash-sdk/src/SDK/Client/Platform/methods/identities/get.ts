// @ts-ignore
import loadWasmDpp from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

// TODO(wasm): import Identifier from wasm-dpp to use as a type
let Identifier;

/**
 * Get an identity from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string|Identifier} id - id
 * @returns Identity
 */
export async function get(this: Platform, id: typeof Identifier | string): Promise<any> {
  await this.initialize();

  // TODO(wasm): expose Metadata from dedicated module that handles all WASM-DPP types
  const { Metadata } = await loadWasmDpp();

  let identityResponse;
  try {
    // TODO: Identifier/buffer issue - hidden Identifier bug.
    //  Without toBuffer() it most probably wont
    //  be recognized on DAPI client side
    identityResponse = await this.client.getDAPIClient().platform
      .getIdentity(id.toBuffer());
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const identity = this.wasmDpp.identity.createFromBuffer(identityResponse.getIdentity());

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

  // TODO(wasm): handle optional metadata in Identity Wasm side
  identity.setMetadata(metadata);

  return identity;
}

export default get;
