// @ts-ignore
import loadWasmDpp from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

let Identifier;
let Metadata;

declare type ContractIdentifier = string | typeof Identifier;

/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @returns contracts
 */
export async function get(this: Platform, identifier: ContractIdentifier): Promise<any> {
  await this.initialize();

  // TODO(wasm): expose Metadata from dedicated module that handles all WASM-DPP types
  ({ Metadata, Identifier } = await loadWasmDpp());

  // TODO: Identifier/buffer issue - hidden Identifier bug.
  //  Without Buffer.from(identifier.toBuffer()) will throw an error

  const contractId : typeof Identifier = Identifier.from(identifier.toBuffer());

  // Try to get contract from the cache
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    // TODO: Identifier/buffer issue - hidden Identifier bug.
    //  Without Buffer.from(contractId.toBuffer()) will throw an error
    if (appDefinition.contractId.equals(contractId.toBuffer()) && appDefinition.contract) {
      return appDefinition.contract;
    }
  }

  // Fetch contract otherwise
  let dataContractResponse;
  try {
    // TODO: Identifier/buffer issue - hidden Identifier bug.
    //  Without .toBuffer() will throw an error
    dataContractResponse = await this.client.getDAPIClient()
      .platform.getDataContract(contractId.toBuffer());
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const contract = await this.wasmDpp.dataContract
    .createFromBuffer(dataContractResponse.getDataContract());

  let metadata = null;
  const responseMetadata = dataContractResponse.getMetadata();
  if (responseMetadata) {
    metadata = new Metadata({
      blockHeight: responseMetadata.getHeight(),
      coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
      timeMs: responseMetadata.getTimeMs(),
      protocolVersion: responseMetadata.getProtocolVersion(),
    });
  }
  contract.setMetadata(metadata);

  // Store contract to the cache
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    // TODO: Identifier/buffer issue - hidden Identifier bug.
    //  Without Buffer.from(contractId.toBuffer()) will throw an error
    if (appDefinition.contractId.equals(contractId.toBuffer())) {
      appDefinition.contract = contract;
    }
  }

  return contract;
}

export default get;
